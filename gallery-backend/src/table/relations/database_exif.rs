use anyhow::Result;
use arrayvec::ArrayString;
use redb::{ReadableTable, TableDefinition};
use std::collections::{BTreeMap, HashMap};

// Key: (Hash, Tag) -> Value: ExifValue (String)
pub const DATABASE_EXIF_TABLE: TableDefinition<(&str, &str), &str> =
    TableDefinition::new("database_exif");

pub struct DatabaseExif;

impl DatabaseExif {
    /// 根據 Hash 取得 EXIF Map
    pub fn fetch_exif(txn: &redb::ReadTransaction, hash: &str) -> Result<BTreeMap<String, String>> {
        let table = txn.open_table(DATABASE_EXIF_TABLE)?;
        let start = (hash, "");
        let end = (hash, "\u{ffff}");

        let mut exif_map = BTreeMap::new();
        for entry in table.range(start..=end)? {
            let ((_, tag), value) = entry?;
            exif_map.insert(tag.to_string(), value.value().to_string());
        }
        Ok(exif_map)
    }

    /// 批次取得所有 EXIF
    pub fn fetch_all_exif(
        txn: &redb::ReadTransaction,
    ) -> Result<HashMap<ArrayString<64>, BTreeMap<String, String>>> {
        let table = txn.open_table(DATABASE_EXIF_TABLE)?;
        let mut map: HashMap<ArrayString<64>, BTreeMap<String, String>> = HashMap::new();

        for entry in table.range::<(&str, &str)>(..)? {
            let ((hash, tag), value) = entry?;
            if let Ok(h) = ArrayString::from(hash) {
                map.entry(h)
                    .or_default()
                    .insert(tag.to_string(), value.value().to_string());
            }
        }
        Ok(map)
    }
}
