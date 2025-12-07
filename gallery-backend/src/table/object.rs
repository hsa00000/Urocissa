use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use sea_query::{ColumnDef, Expr, ExprTrait, Iden, Index, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectType {
    Image,
    Video,
    Album,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Image => write!(f, "image"),
            ObjectType::Video => write!(f, "video"),
            ObjectType::Album => write!(f, "album"),
        }
    }
}

// [Add] 實作 FromStr 以便從字串轉換為 Enum (從資料庫讀取用)
impl FromStr for ObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image" => Ok(ObjectType::Image),
            "video" => Ok(ObjectType::Video),
            "album" => Ok(ObjectType::Album),
            _ => Err(format!("Invalid ObjectType: {}", s)),
        }
    }
}

impl ObjectType {
    /// 根據副檔名判斷類型
    pub fn from_ext(ext: impl AsRef<str>) -> Option<Self> {
        let ext = ext.as_ref();
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Image)
        } else if VALID_VIDEO_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Video)
        } else {
            None
        }
    }
}

// SeaQuery Table Definition
#[derive(Iden)]
pub enum Object {
    Table,
    Id,
    ObjType,
    CreatedTime,
    Pending,
    Thumbhash,
}

/// ObjectSchema: 系統中所有實體的共同基類
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: ObjectType,
    pub created_time: i64,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub tags: HashSet<String>,
}

impl ObjectSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // 1. Create Table
        let sql = Table::create()
            .table(Object::Table)
            .if_not_exists()
            .col(ColumnDef::new(Object::Id).text().primary_key())
            .col(
                ColumnDef::new(Object::ObjType)
                    .text()
                    .not_null()
                    .check(Expr::col(Object::ObjType).is_in(["image", "video", "album"])),
            )
            .col(ColumnDef::new(Object::CreatedTime).integer().not_null())
            .col(ColumnDef::new(Object::Pending).integer().default(0))
            .col(ColumnDef::new(Object::Thumbhash).blob())
            .build(SqliteQueryBuilder);

        conn.execute(&sql, [])?;

        // 2. Create Indexes
        let idx_time = Index::create()
            .if_not_exists()
            .name("idx_object_created_time")
            .table(Object::Table)
            .col(Object::CreatedTime)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx_time, [])?;

        let idx_type = Index::create()
            .if_not_exists()
            .name("idx_object_type")
            .table(Object::Table)
            .col(Object::ObjType)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx_type, [])?;

        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        // [Modify] 從 DB 讀取字串並轉換為 ObjectType
        let obj_type_str: String = row.get(Object::ObjType.to_string().as_str())?;
        let obj_type = ObjectType::from_str(&obj_type_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        let id_str: String = row.get(Object::Id.to_string().as_str())?;

        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            obj_type, // [Modify] 使用轉換後的 enum
            created_time: row.get(Object::CreatedTime.to_string().as_str())?,
            pending: row.get(Object::Pending.to_string().as_str())?,
            thumbhash: row.get(Object::Thumbhash.to_string().as_str())?,
            tags: HashSet::new(),
        })
    }

    pub fn new(id: ArrayString<64>, obj_type: ObjectType) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            obj_type,
            created_time: timestamp,
            pending: false,
            thumbhash: None,
            tags: HashSet::new(),
        }
    }
}
