use redb::Builder;

const MIB: usize = 1024 * 1024;

pub const MAIN_CACHE_BYTES: usize = 128 * MIB;
pub const TREE_SNAPSHOT_CACHE_BYTES: usize = 32 * MIB;
pub const QUERY_SNAPSHOT_CACHE_BYTES: usize = 16 * MIB;
pub const EXPIRE_CACHE_BYTES: usize = 8 * MIB;
pub const MIGRATION_CACHE_BYTES: usize = 128 * MIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheClass {
    Main,
    TreeSnapshot,
    QuerySnapshot,
    Expire,
    Migration,
}

impl CacheClass {
    pub const fn limit_bytes(self) -> usize {
        match self {
            Self::Main => MAIN_CACHE_BYTES,
            Self::TreeSnapshot => TREE_SNAPSHOT_CACHE_BYTES,
            Self::QuerySnapshot => QUERY_SNAPSHOT_CACHE_BYTES,
            Self::Expire => EXPIRE_CACHE_BYTES,
            Self::Migration => MIGRATION_CACHE_BYTES,
        }
    }
}

pub fn database_builder(cache_class: CacheClass) -> Builder {
    let mut builder = redb::Database::builder();
    builder.set_cache_size(cache_class.limit_bytes());
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_budgets_match_the_four_gib_policy() {
        assert_eq!(CacheClass::Main.limit_bytes(), 128 * MIB);
        assert_eq!(CacheClass::TreeSnapshot.limit_bytes(), 32 * MIB);
        assert_eq!(CacheClass::QuerySnapshot.limit_bytes(), 16 * MIB);
        assert_eq!(CacheClass::Expire.limit_bytes(), 8 * MIB);
        assert_eq!(CacheClass::Migration.limit_bytes(), 128 * MIB);
        assert_eq!(
            TREE_SNAPSHOT_CACHE_BYTES + QUERY_SNAPSHOT_CACHE_BYTES + EXPIRE_CACHE_BYTES,
            56 * MIB
        );
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn cache_usage_stays_within_the_selected_class_limit() {
        use redb::{ReadableDatabase, TableDefinition};

        let directory = tempfile::tempdir().unwrap();
        let database = database_builder(CacheClass::Expire)
            .create(directory.path().join("cache-limit.redb"))
            .unwrap();
        let table = TableDefinition::<u64, &[u8]>::new("values");
        let transaction = database.begin_write().unwrap();
        {
            let mut values = transaction.open_table(table).unwrap();
            let payload = vec![0x5a; 4 * 1024];
            for key in 0..512 {
                values.insert(key, payload.as_slice()).unwrap();
            }
        }
        transaction.commit().unwrap();

        assert!(database.cache_stats().used_bytes() <= EXPIRE_CACHE_BYTES);
    }
}
