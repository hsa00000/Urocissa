use anyhow::Result;
use arrayvec::ArrayString;
use redb::{ReadTransaction, TableDefinition};
use std::collections::{BTreeMap, HashMap};

// Key: (Hash, Tag) -> Value: ExifValue (String)
pub const DATABASE_EXIF_TABLE: TableDefinition<(&str, &str), &str> =
    TableDefinition::new("database_exif");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExifSchema {
    pub hash: String,
    pub tag: String,
    pub value: String,
}

pub struct DatabaseExif;

impl DatabaseExif {
    pub fn fetch_exif(
        txn: &ReadTransaction,
        hash: &str,
    ) -> Result<BTreeMap<String, String>> {
        let table = txn.open_table(DATABASE_EXIF_TABLE)?;
        let start = (hash, "");
        let end = (hash, "\u{ffff}");

        let mut exif_map = BTreeMap::new();
        for entry in table.range(start..=end)? {
            let (key_guard, value) = entry?;
            let key = key_guard.value();
            let (_, tag) = key;
            exif_map.insert(tag.to_string(), value.value().to_string());
        }
        Ok(exif_map)
    }

    pub fn fetch_all_exif(
        txn: &ReadTransaction,
    ) -> Result<HashMap<ArrayString<64>, BTreeMap<String, String>>> {
        let table = txn.open_table(DATABASE_EXIF_TABLE)?;
        let mut map: HashMap<ArrayString<64>, BTreeMap<String, String>> = HashMap::new();

        for entry in table.range::<(&str, &str)>(..)? {
            let (key_guard, value) = entry?;
            let key = key_guard.value();
            let (hash, tag) = key;
            if let Ok(h) = ArrayString::from(hash) {
                map.entry(h)
                    .or_default()
                    .insert(tag.to_string(), value.value().to_string());
            }
        }
        Ok(map)
    }
}
