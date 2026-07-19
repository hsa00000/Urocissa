use std::sync::{Arc, LazyLock, Weak};

use arrayvec::ArrayString;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use tokio::sync::{Mutex, OwnedMutexGuard};

static MEDIA_LOCKS: LazyLock<DashMap<ArrayString<64>, Weak<Mutex<()>>>> =
    LazyLock::new(DashMap::new);

pub struct MediaLockGuard {
    id: ArrayString<64>,
    weak: Weak<Mutex<()>>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for MediaLockGuard {
    fn drop(&mut self) {
        // Release the Tokio lock before testing whether this was the last
        // strong reference. Waiting mutations keep their own Arc alive.
        drop(self.guard.take());
        if let Entry::Occupied(entry) = MEDIA_LOCKS.entry(self.id)
            && Weak::ptr_eq(entry.get(), &self.weak)
            && entry.get().strong_count() == 0
        {
            entry.remove();
        }
    }
}

/// Serialize every mutation of one canonical media object across imports,
/// reindex jobs, rotation and captured-frame thumbnail changes.
pub async fn lock_media(id: ArrayString<64>) -> MediaLockGuard {
    let lock = match MEDIA_LOCKS.entry(id) {
        Entry::Occupied(mut entry) => entry.get().upgrade().unwrap_or_else(|| {
            let lock = Arc::new(Mutex::new(()));
            entry.insert(Arc::downgrade(&lock));
            lock
        }),
        Entry::Vacant(entry) => {
            let lock = Arc::new(Mutex::new(()));
            entry.insert(Arc::downgrade(&lock));
            lock
        }
    };
    let weak = Arc::downgrade(&lock);
    let guard = lock.lock_owned().await;
    MediaLockGuard {
        id,
        weak,
        guard: Some(guard),
    }
}

#[cfg(test)]
mod tests {
    use super::{MEDIA_LOCKS, lock_media};

    #[tokio::test]
    async fn keyed_lock_is_removed_after_the_last_waiter() {
        let id = arrayvec::ArrayString::from("media-lock-test").unwrap();
        let first = lock_media(id).await;
        let waiter = tokio::spawn(async move { lock_media(id).await });
        tokio::task::yield_now().await;
        assert!(MEDIA_LOCKS.contains_key(&id));
        drop(first);
        let second = waiter.await.unwrap();
        assert!(MEDIA_LOCKS.contains_key(&id));
        drop(second);
        assert!(!MEDIA_LOCKS.contains_key(&id));
    }
}
