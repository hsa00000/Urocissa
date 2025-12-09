use rusqlite::Result;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

/// 用來讓 u64 可以無損存入 SQLite 的 i64 欄位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SqliteU64(pub u64);

impl ToSql for SqliteU64 {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        // 直接將位元解釋為 i64 (Bitwise cast)
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for SqliteU64 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        // 讀出來是 i64，再轉回 u64
        let i = i64::column_result(value)?;
        Ok(SqliteU64(i as u64))
    }
}

// 方便轉換
impl From<u64> for SqliteU64 {
    fn from(v: u64) -> Self {
        Self(v)
    }
}
