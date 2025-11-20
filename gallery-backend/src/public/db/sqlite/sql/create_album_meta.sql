CREATE TABLE IF NOT EXISTS album_meta (
    album_id TEXT PRIMARY KEY,
    cover_id TEXT,
    user_defined_metadata TEXT NOT NULL DEFAULT '{}',
    item_count INTEGER NOT NULL DEFAULT 0,
    item_size INTEGER NOT NULL DEFAULT 0,
    start_time INTEGER,
    end_time INTEGER,
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE,
    FOREIGN KEY (cover_id) REFERENCES nodes (id)
);

CREATE TRIGGER IF NOT EXISTS check_album_kind_insert BEFORE INSERT ON album_meta FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN (
            SELECT
                kind
            FROM
                nodes
            WHERE
                id = NEW.album_id
        ) != 'album' THEN RAISE (
            ABORT,
            'album_id must reference a node with kind = album'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS check_album_kind_update BEFORE
UPDATE ON album_meta FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN (
            SELECT
                kind
            FROM
                nodes
            WHERE
                id = NEW.album_id
        ) != 'album' THEN RAISE (
            ABORT,
            'album_id must reference a node with kind = album'
        )
    END;

END