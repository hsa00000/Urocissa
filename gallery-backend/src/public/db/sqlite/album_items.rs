use rusqlite::{Connection, params};

pub fn create_album_items_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS album_items (
            album_id TEXT,
            item_id TEXT,
            PRIMARY KEY (album_id, item_id),
            FOREIGN KEY (album_id) REFERENCES nodes(id) ON DELETE CASCADE,
            FOREIGN KEY (item_id) REFERENCES nodes(id) ON DELETE CASCADE
        )",
        [],
    ).map(|_| ())
}

pub fn create_triggers(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TRIGGER IF NOT EXISTS trg_album_items_ai
        AFTER INSERT ON album_items
        BEGIN
            UPDATE album_meta
            SET
                item_count = item_count + 1,
                item_size  = item_size + COALESCE(
                    (SELECT size FROM nodes WHERE id = NEW.item_id),
                    0
                )
            WHERE album_id = NEW.album_id;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_album_items_ad
        AFTER DELETE ON album_items
        BEGIN
            UPDATE album_meta
            SET
                item_count = item_count - 1,
                item_size  = item_size - COALESCE(
                    (SELECT size FROM nodes WHERE id = OLD.item_id),
                    0
                )
            WHERE album_id = OLD.album_id;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_album_items_au
        AFTER UPDATE OF item_id ON album_items
        BEGIN
            UPDATE album_meta
            SET item_size = item_size - COALESCE(
                (SELECT size FROM nodes WHERE id = OLD.item_id),
                0
            )
            WHERE album_id = NEW.album_id;

            UPDATE album_meta
            SET item_size = item_size + COALESCE(
                (SELECT size FROM nodes WHERE id = NEW.item_id),
                0
            )
            WHERE album_id = NEW.album_id;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_nodes_size_au
        AFTER UPDATE OF size ON nodes
        WHEN NEW.size != OLD.size
        BEGIN
            UPDATE album_meta
            SET item_size = item_size + (NEW.size - OLD.size)
            WHERE album_id IN (
                SELECT album_id
                FROM album_items
                WHERE item_id = NEW.id
            );
        END;
        "
    )
}

pub fn _get_objects_in_album(conn: &Connection, album_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT item_id FROM album_items WHERE album_id = ?")?;
    let iter = stmt.query_map(params![album_id], |row| row.get(0))?;
    let mut ids = Vec::new();
    for id in iter {
        ids.push(id?);
    }
    Ok(ids)
}