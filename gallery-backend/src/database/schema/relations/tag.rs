use anyhow::Result;
use arrayvec::ArrayString;
use redb::{ReadTransaction, TableDefinition, WriteTransaction};
use std::collections::{HashMap, HashSet};

// 正向: (Hash, Tag) -> ()
pub const TAG_DATABASE_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("rel_object_tags");

// 反向索引: (Tag, Hash) -> ()
pub const IDX_TAG_HASH_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("idx_tags_object");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDatabaseSchema {
    pub hash: String,
    pub tag: String,
}

pub struct TagDatabase;

impl TagDatabase {
    pub fn add_tag(txn: &mut WriteTransaction, hash: &str, tag: &str) -> Result<()> {
        txn.open_table(TAG_DATABASE_TABLE)?
            .insert((hash, tag), &())?;
        txn.open_table(IDX_TAG_HASH_TABLE)?
            .insert((tag, hash), &())?;
        Ok(())
    }

    pub fn remove_tag(txn: &mut WriteTransaction, hash: &str, tag: &str) -> Result<()> {
        txn.open_table(TAG_DATABASE_TABLE)?.remove((hash, tag))?;
        txn.open_table(IDX_TAG_HASH_TABLE)?.remove((tag, hash))?;
        Ok(())
    }

    pub fn fetch_tags(txn: &ReadTransaction, hash: &str) -> Result<HashSet<String>> {
        let table = txn.open_table(TAG_DATABASE_TABLE)?;
        let start = (hash, "");

        let mut tags = HashSet::new();
        // 使用 start.. 進行範圍掃描，並在遇到不同的 Hash 時中斷
        for entry in table.range(start..)? {
            let (key_guard, _) = entry?;
            let (key_hash, tag) = key_guard.value();

            if key_hash != hash {
                break;
            }

            tags.insert(tag.to_string());
        }
        Ok(tags)
    }

    pub fn fetch_all_tags(
        txn: &ReadTransaction,
    ) -> Result<HashMap<ArrayString<64>, HashSet<String>>> {
        let table = txn.open_table(TAG_DATABASE_TABLE)?;
        let mut map: HashMap<ArrayString<64>, HashSet<String>> = HashMap::new();

        for entry in table.range::<(&str, &str)>(..)? {
            let (key_guard, _) = entry?;
            let key = key_guard.value();
            let (hash, tag) = key;
            if let Ok(h) = ArrayString::from(hash) {
                map.entry(h).or_default().insert(tag.to_string());
            }
        }
        Ok(map)
    }
}
