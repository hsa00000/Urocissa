use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use anyhow::{Context, Result, anyhow};

#[derive(Debug)]
enum ArtifactAction {
    Replace {
        staged: PathBuf,
        destination: PathBuf,
    },
    Remove {
        destination: PathBuf,
    },
}

#[derive(Debug)]
struct AppliedArtifact {
    destination: PathBuf,
    backup: PathBuf,
    installed_new_file: bool,
}

/// A process-local artifact transaction. Staged files and backups are siblings
/// of the destination so every rename stays on one filesystem.
#[derive(Debug)]
pub struct ArtifactPublisher {
    token: String,
    actions: Vec<ArtifactAction>,
    retire_after_commit: Vec<PathBuf>,
}

impl ArtifactPublisher {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            actions: Vec::new(),
            retire_after_commit: Vec::new(),
        }
    }

    pub fn stage_path(&self, destination: &Path) -> Result<PathBuf> {
        sibling_path(destination, &self.token, "tmp")
    }

    pub fn replace(&mut self, staged: PathBuf, destination: PathBuf) {
        self.actions.push(ArtifactAction::Replace {
            staged,
            destination,
        });
    }

    pub fn remove(&mut self, destination: PathBuf) {
        self.actions.push(ArtifactAction::Remove { destination });
    }

    /// Remove an obsolete immutable artifact only after its replacement and
    /// the durable metadata commit both succeed.
    pub fn retire_after_commit(&mut self, destination: PathBuf) {
        self.retire_after_commit.push(destination);
    }

    /// Promote artifacts, execute the durable/cache commit, then discard
    /// backups. Any in-process failure restores every destination.
    pub fn publish<T>(self, commit: impl FnOnce() -> Result<T>) -> Result<T> {
        let mut applied = Vec::with_capacity(self.actions.len());
        if let Err(error) = self.promote(&mut applied) {
            rollback(&applied);
            self.cleanup_staged();
            return Err(error);
        }

        match commit() {
            Ok(value) => {
                for artifact in &applied {
                    if artifact.backup.exists()
                        && let Err(error) = fs::remove_file(&artifact.backup)
                    {
                        log::warn!(
                            "failed to remove media artifact backup {}: {error}",
                            artifact.backup.display()
                        );
                    }
                }
                for destination in &self.retire_after_commit {
                    if destination.exists()
                        && let Err(error) = fs::remove_file(destination)
                    {
                        log::warn!(
                            "failed to retire obsolete media artifact {}: {error}",
                            destination.display()
                        );
                    }
                }
                Ok(value)
            }
            Err(error) => {
                rollback(&applied);
                Err(error)
            }
        }
    }

    fn promote(&self, applied: &mut Vec<AppliedArtifact>) -> Result<()> {
        for action in &self.actions {
            let destination = match action {
                ArtifactAction::Replace { destination, .. }
                | ArtifactAction::Remove { destination } => destination,
            };
            let parent = destination.parent().ok_or_else(|| {
                anyhow!(
                    "artifact destination has no parent: {}",
                    destination.display()
                )
            })?;
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create artifact directory {}", parent.display())
            })?;

            let backup = sibling_path(destination, &self.token, "bak")?;
            if backup.exists() {
                fs::remove_file(&backup).with_context(|| {
                    format!(
                        "failed to remove stale artifact backup {}",
                        backup.display()
                    )
                })?;
            }
            if destination.exists() {
                fs::rename(destination, &backup).with_context(|| {
                    format!(
                        "failed to back up artifact {} to {}",
                        destination.display(),
                        backup.display()
                    )
                })?;
            }

            let installed_new_file = match action {
                ArtifactAction::Replace { staged, .. } => {
                    if !staged.exists() {
                        if backup.exists() {
                            let _ = fs::rename(&backup, destination);
                        }
                        return Err(anyhow!("staged artifact is missing: {}", staged.display()));
                    }
                    if let Err(error) = fs::rename(staged, destination) {
                        if backup.exists() {
                            let _ = fs::rename(&backup, destination);
                        }
                        return Err(anyhow!(error).context(format!(
                            "failed to promote artifact {} to {}",
                            staged.display(),
                            destination.display()
                        )));
                    }
                    true
                }
                ArtifactAction::Remove { .. } => false,
            };
            applied.push(AppliedArtifact {
                destination: destination.clone(),
                backup,
                installed_new_file,
            });
        }
        Ok(())
    }

    fn cleanup_staged(&self) {
        for action in &self.actions {
            if let ArtifactAction::Replace { staged, .. } = action
                && staged.exists()
            {
                let _ = fs::remove_file(staged);
            }
        }
    }
}

impl Drop for ArtifactPublisher {
    fn drop(&mut self) {
        self.cleanup_staged();
    }
}

fn rollback(applied: &[AppliedArtifact]) {
    for artifact in applied.iter().rev() {
        if artifact.installed_new_file && artifact.destination.exists() {
            let _ = fs::remove_file(&artifact.destination);
        }
        if artifact.backup.exists()
            && let Err(error) = fs::rename(&artifact.backup, &artifact.destination)
        {
            log::error!(
                "failed to restore media artifact {}: {error}",
                artifact.destination.display()
            );
        }
    }
}

fn sibling_path(destination: &Path, token: &str, kind: &str) -> Result<PathBuf> {
    let parent = destination.parent().ok_or_else(|| {
        anyhow!(
            "artifact destination has no parent: {}",
            destination.display()
        )
    })?;
    let stem = destination
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("artifact destination has no UTF-8 stem"))?;
    let file_name = destination
        .extension()
        .and_then(|value| value.to_str())
        .map_or_else(
            || format!("{stem}.{token}.{kind}"),
            |extension| format!("{stem}.{token}.{kind}.{extension}"),
        );
    Ok(parent.join(file_name))
}

/// Filesystem cleanup statistics emitted after startup reconciliation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactCleanupSummary {
    pub scanned: usize,
    pub removed: usize,
    pub skipped_new: usize,
    pub errors: usize,
}

/// Reconcile immutable JPEG thumbnails and crash leftovers in one shard scan.
/// The cutoff prevents this startup task from deleting files published after
/// readiness while the watcher and write-behind workers are starting.
pub fn cleanup_startup_artifacts(
    root: &Path,
    expected: &HashMap<String, u32>,
    cutoff: SystemTime,
) -> ArtifactCleanupSummary {
    let started = Instant::now();
    let mut summary = ArtifactCleanupSummary::default();
    log::info!(
        "Background artifact cleanup started: root {}, expected media {}, cutoff {:?}",
        root.display(),
        expected.len(),
        cutoff
    );
    let shards = match fs::read_dir(root) {
        Ok(shards) => shards,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::perf_timing!(
                "startup.artifact_cleanup",
                started,
                "Background artifact cleanup finished: root {} does not exist",
                root.display()
            );
            return summary;
        }
        Err(error) => {
            log::warn!(
                "failed to scan compressed object root {}: {error}",
                root.display()
            );
            summary.errors += 1;
            return summary;
        }
    };

    for shard_result in shards {
        let shard = match shard_result {
            Ok(shard) => shard,
            Err(error) => {
                summary.errors += 1;
                log::warn!("failed to read compressed object directory entry: {error}");
                continue;
            }
        };
        let is_directory = match shard.file_type() {
            Ok(file_type) => file_type.is_dir(),
            Err(error) => {
                summary.errors += 1;
                log::warn!(
                    "failed to inspect compressed object directory {}: {error}",
                    shard.path().display()
                );
                false
            }
        };
        if !is_directory {
            continue;
        }

        let entries = match fs::read_dir(shard.path()) {
            Ok(entries) => entries,
            Err(error) => {
                summary.errors += 1;
                log::warn!(
                    "failed to scan compressed object shard {}: {error}",
                    shard.path().display()
                );
                continue;
            }
        };
        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(error) => {
                    summary.errors += 1;
                    log::warn!("failed to read compressed object entry: {error}");
                    continue;
                }
            };
            summary.scanned += 1;
            if summary.scanned.is_multiple_of(25_000) {
                log::info!(
                    "Background artifact cleanup scanned {} files (removed {})",
                    summary.scanned,
                    summary.removed
                );
            }

            let is_file = match entry.file_type() {
                Ok(file_type) => file_type.is_file(),
                Err(error) => {
                    summary.errors += 1;
                    log::warn!(
                        "failed to inspect compressed object {}: {error}",
                        entry.path().display()
                    );
                    false
                }
            };
            if !is_file {
                continue;
            }

            let path = entry.path();
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let stale_thumbnail = path.extension().and_then(|extension| extension.to_str())
                == Some("jpg")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(crate::public::structure::abstract_data::parse_thumbnail_stem)
                    .is_some_and(|(hash, cache_version)| {
                        !expected
                            .get(hash)
                            .is_some_and(|expected| *expected == cache_version)
                    });
            let residual = is_residual_artifact(&file_name);
            if !stale_thumbnail && !residual {
                continue;
            }

            let modified = match entry.metadata().and_then(|metadata| metadata.modified()) {
                Ok(modified) => modified,
                Err(error) => {
                    summary.errors += 1;
                    log::warn!(
                        "failed to read modification time for {}: {error}",
                        path.display()
                    );
                    continue;
                }
            };
            if modified > cutoff {
                summary.skipped_new += 1;
                continue;
            }

            match fs::remove_file(&path) {
                Ok(()) => summary.removed += 1,
                Err(error) => {
                    summary.errors += 1;
                    log::warn!(
                        "failed to remove startup artifact {}: {error}",
                        path.display()
                    );
                }
            }
        }
    }

    crate::perf_timing!(
        "startup.artifact_cleanup",
        started,
        "Background artifact cleanup finished: scanned {}, removed {}, skipped new {}, errors {}",
        summary.scanned,
        summary.removed,
        summary.skipped_new,
        summary.errors
    );
    summary
}

fn is_residual_artifact(file_name: &str) -> bool {
    let parts = file_name.split('.').collect::<Vec<_>>();
    parts.len() == 4
        && crate::public::structure::abstract_data::parse_thumbnail_stem(parts[0]).is_some()
        && matches!(parts[2], "tmp" | "bak")
        && (parts[1].starts_with("reindex-")
            || parts[1].starts_with("import-")
            || parts[1].starts_with("rotate-")
            || parts[1].starts_with("capture-"))
}

#[cfg(test)]
fn cleanup_stale_thumbnail_versions_at(root: &Path, expected: &HashMap<String, u32>) -> usize {
    cleanup_startup_artifacts(root, expected, SystemTime::now()).removed
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        ArtifactPublisher, cleanup_stale_thumbnail_versions_at, cleanup_startup_artifacts,
    };

    #[test]
    fn staged_file_keeps_destination_extension() {
        let publisher = ArtifactPublisher::new("job-1");
        let staged = publisher
            .stage_path(std::path::Path::new("C:/data/example.jpg"))
            .unwrap();
        assert_eq!(staged.file_name().unwrap(), "example.job-1.tmp.jpg");
    }

    #[test]
    fn failed_commit_restores_previous_artifact() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("object.jpg");
        std::fs::write(&destination, b"old").unwrap();
        let mut publisher = ArtifactPublisher::new("job-2");
        let staged = publisher.stage_path(&destination).unwrap();
        std::fs::write(&staged, b"new").unwrap();
        publisher.replace(staged, destination.clone());
        let result: anyhow::Result<()> = publisher.publish(|| anyhow::bail!("database failed"));
        assert!(result.is_err());
        assert_eq!(std::fs::read(destination).unwrap(), b"old");
    }

    #[test]
    fn failed_artifact_promotion_restores_previous_artifact() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("object.jpg");
        std::fs::write(&destination, b"old").unwrap();
        let mut publisher = ArtifactPublisher::new("job-missing");
        let missing = publisher.stage_path(&destination).unwrap();
        publisher.replace(missing, destination.clone());

        let result = publisher.publish(|| Ok(()));

        assert!(result.is_err());
        assert_eq!(std::fs::read(destination).unwrap(), b"old");
    }

    #[test]
    fn successful_commit_keeps_new_artifact_and_removes_backup() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("object.jpg");
        std::fs::write(&destination, b"old").unwrap();
        let mut publisher = ArtifactPublisher::new("job-3");
        let staged = publisher.stage_path(&destination).unwrap();
        std::fs::write(&staged, b"new").unwrap();
        publisher.replace(staged, destination.clone());
        publisher.publish(|| Ok(())).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new");
        assert!(!directory.path().join("object.job-3.bak.jpg").exists());
    }

    #[test]
    fn failed_version_commit_keeps_old_thumbnail_and_rolls_back_new_version() {
        let directory = tempfile::tempdir().unwrap();
        let old_thumbnail = directory.path().join("hash.jpg");
        let new_thumbnail = directory.path().join("hash-v1.jpg");
        std::fs::write(&old_thumbnail, b"old").unwrap();
        let mut publisher = ArtifactPublisher::new("job-version-failed");
        let staged = publisher.stage_path(&new_thumbnail).unwrap();
        std::fs::write(&staged, b"new").unwrap();
        publisher.replace(staged, new_thumbnail.clone());
        publisher.retire_after_commit(old_thumbnail.clone());

        let result: anyhow::Result<()> = publisher.publish(|| anyhow::bail!("database failed"));

        assert!(result.is_err());
        assert_eq!(std::fs::read(old_thumbnail).unwrap(), b"old");
        assert!(!new_thumbnail.exists());
    }

    #[test]
    fn successful_version_commit_retires_old_thumbnail() {
        let directory = tempfile::tempdir().unwrap();
        let old_thumbnail = directory.path().join("hash.jpg");
        let new_thumbnail = directory.path().join("hash-v1.jpg");
        std::fs::write(&old_thumbnail, b"old").unwrap();
        let mut publisher = ArtifactPublisher::new("job-version-success");
        let staged = publisher.stage_path(&new_thumbnail).unwrap();
        std::fs::write(&staged, b"new").unwrap();
        publisher.replace(staged, new_thumbnail.clone());
        publisher.retire_after_commit(old_thumbnail.clone());

        publisher.publish(|| Ok(())).unwrap();

        assert!(!old_thumbnail.exists());
        assert_eq!(std::fs::read(new_thumbnail).unwrap(), b"new");
    }

    #[test]
    fn startup_cleanup_keeps_only_the_durable_thumbnail_version() {
        const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        const ORPHAN: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let directory = tempfile::tempdir().unwrap();
        let shard = directory.path().join("01");
        std::fs::create_dir_all(&shard).unwrap();
        let current = shard.join(format!("{HASH}-v2.jpg"));
        let old_v0 = shard.join(format!("{HASH}.jpg"));
        let old_v1 = shard.join(format!("{HASH}-v1.jpg"));
        let orphan = shard.join(format!("{ORPHAN}-v4.jpg"));
        let residual = shard.join(format!("{HASH}-v2.rotate-job.tmp.jpg"));
        let unrelated = shard.join("not-a-thumbnail.jpg");
        for path in [&current, &old_v0, &old_v1, &orphan, &residual, &unrelated] {
            std::fs::write(path, b"jpeg").unwrap();
        }
        let expected = HashMap::from([(HASH.to_owned(), 2)]);

        let removed = cleanup_stale_thumbnail_versions_at(directory.path(), &expected);

        assert_eq!(removed, 4);
        assert!(current.exists());
        assert!(!old_v0.exists());
        assert!(!old_v1.exists());
        assert!(!orphan.exists());
        assert!(!residual.exists());
        assert!(unrelated.exists());
    }

    #[test]
    fn startup_cleanup_skips_files_created_after_cutoff() {
        const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let directory = tempfile::tempdir().unwrap();
        let shard = directory.path().join("01");
        std::fs::create_dir_all(&shard).unwrap();
        let stale = shard.join(format!("{HASH}-v1.jpg"));
        std::fs::write(&stale, b"jpeg").unwrap();
        let expected = HashMap::from([(HASH.to_owned(), 0)]);

        let summary = cleanup_startup_artifacts(
            directory.path(),
            &expected,
            std::time::SystemTime::UNIX_EPOCH,
        );

        assert_eq!(summary.removed, 0);
        assert_eq!(summary.skipped_new, 1);
        assert!(stale.exists());
    }
}
