use crate::public::structure::expression::Expression;
use rusqlite::{Connection, params, OptionalExtension, ToSql};

const CREATE_SNAPSHOTS_SQL: &str = include_str!("sql/create_snapshots.sql");

pub fn create_snapshots_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_SNAPSHOTS_SQL, []).map(|_| ())
}

pub fn get_snapshot_len(conn: &Connection, timestamp: u128) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM snapshots WHERE timestamp = ?")?;
    stmt.query_row(params![timestamp as i64], |row| row.get(0))
}

pub fn get_snapshot_hash(conn: &Connection, timestamp: u128, idx: usize) -> rusqlite::Result<String> {
    let mut stmt =
        conn.prepare("SELECT node_id FROM snapshots WHERE timestamp = ? AND idx = ?")?;
    stmt.query_row(params![timestamp as i64, idx], |row| row.get(0))
}

pub fn get_snapshot_width_height(
    conn: &Connection,
    timestamp: u128,
    idx: usize,
) -> rusqlite::Result<(u32, u32)> {
    let mut stmt = conn.prepare(
        "SELECT nodes.width, nodes.height
         FROM snapshots s
         JOIN nodes ON s.node_id = nodes.id
         WHERE s.timestamp = ? AND s.idx = ?",
    )?;
    
    stmt.query_row(params![timestamp as i64, idx], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })
}

pub fn get_snapshot_dates(conn: &Connection, timestamp: u128) -> rusqlite::Result<Vec<(usize, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT s.idx, nodes.timestamp
         FROM snapshots s
         JOIN nodes ON s.node_id = nodes.id
         WHERE s.timestamp = ?
         ORDER BY s.idx ASC",
    )?;
    
    let iter = stmt.query_map(params![timestamp as i64], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    let mut dates = Vec::new();
    for date in iter {
        dates.push(date?);
    }
    Ok(dates)
}

pub fn generate_snapshot(
    conn: &Connection,
    timestamp: u128,
    expression: &Option<Expression>,
    hide_metadata: bool,
    shared_album_id: Option<&str>,
) -> rusqlite::Result<usize> {
    let (where_clause, params) = if let Some(expr) = expression {
        expr.to_sql(hide_metadata, shared_album_id)
    } else {
        ("1=1".to_string(), vec![])
    };

    // Note: timestamp is cast to i64 for SQLite INTEGER compatibility
    let sql = format!(
        "INSERT INTO snapshots (timestamp, idx, node_id)
         SELECT ?, ROW_NUMBER() OVER (ORDER BY timestamp DESC) - 1, id
         FROM nodes
         WHERE kind IN ('image', 'video') AND {}",
        where_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    // Combine timestamp param with expression params
    let timestamp_i64 = timestamp as i64;
    let mut sql_params: Vec<&dyn ToSql> = vec![&timestamp_i64];
    let params_refs: Vec<&dyn ToSql> = params.iter().map(|p| &**p as &dyn ToSql).collect();
    sql_params.extend(params_refs);

    let count = stmt.execute(sql_params.as_slice())?;
    Ok(count)
}

pub fn get_snapshot_index(
    conn: &Connection,
    timestamp: u128,
    hash: &str,
) -> rusqlite::Result<Option<usize>> {
    let mut stmt =
        conn.prepare("SELECT idx FROM snapshots WHERE timestamp = ? AND node_id = ?")?;
    stmt.query_row(params![timestamp as i64, hash], |row| row.get(0))
        .optional()
}

pub fn delete_expired_snapshots(conn: &Connection, timestamp_threshold: u128) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare("DELETE FROM snapshots WHERE timestamp < ?")?;
    stmt.execute(params![timestamp_threshold as i64])
}