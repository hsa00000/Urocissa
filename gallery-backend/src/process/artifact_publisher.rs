use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use walkdir::WalkDir;

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
}

impl ArtifactPublisher {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            actions: Vec::new(),
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

/// Crash recovery is intentionally limited to removing job-scoped leftovers;
/// jobs and database transactions are not resumed across a restart.
pub fn cleanup_residual_artifacts() {
    let root = crate::public::constant::storage::get_data_path().join("object/compressed");
    let mut removed = 0_usize;
    for entry in WalkDir::new(&root)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Some(file_name) = entry.file_name().to_str() else {
            continue;
        };
        let parts = file_name.split('.').collect::<Vec<_>>();
        if parts.len() != 4
            || parts[0].len() != 64
            || !matches!(parts[2], "tmp" | "bak")
            || !(parts[1].starts_with("reindex-")
                || parts[1].starts_with("import-")
                || parts[1].starts_with("rotate-")
                || parts[1].starts_with("capture-"))
        {
            continue;
        }
        match fs::remove_file(entry.path()) {
            Ok(()) => removed += 1,
            Err(error) => log::warn!(
                "failed to remove residual media artifact {}: {error}",
                entry.path().display()
            ),
        }
    }
    if removed > 0 {
        log::info!("Removed {removed} residual media artifact files");
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactPublisher;

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
}
