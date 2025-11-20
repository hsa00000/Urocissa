CREATE TABLE IF NOT EXISTS album_items (
    album_id TEXT,
    item_id TEXT,
    PRIMARY KEY (album_id, item_id),
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES nodes (id) ON DELETE CASCADE
);

CREATE TRIGGER IF NOT EXISTS trg_album_items_ai AFTER INSERT ON album_items BEGIN
UPDATE album_meta
SET
    item_count = item_count + 1,
    item_size = item_size + COALESCE(
        (
            SELECT
                size
            FROM
                nodes
            WHERE
                id = NEW.item_id
        ),
        0
    )
WHERE
    album_id = NEW.album_id;

END;

CREATE TRIGGER IF NOT EXISTS trg_album_items_ad AFTER DELETE ON album_items BEGIN
UPDATE album_meta
SET
    item_count = item_count - 1,
    item_size = item_size - COALESCE(
        (
            SELECT
                size
            FROM
                nodes
            WHERE
                id = OLD.item_id
        ),
        0
    )
WHERE
    album_id = OLD.album_id;

END;

CREATE TRIGGER IF NOT EXISTS trg_album_items_au AFTER
UPDATE OF item_id ON album_items BEGIN
UPDATE album_meta
SET
    item_size = item_size - COALESCE(
        (
            SELECT
                size
            FROM
                nodes
            WHERE
                id = OLD.item_id
        ),
        0
    )
WHERE
    album_id = NEW.album_id;

UPDATE album_meta
SET
    item_size = item_size + COALESCE(
        (
            SELECT
                size
            FROM
                nodes
            WHERE
                id = NEW.item_id
        ),
        0
    )
WHERE
    album_id = NEW.album_id;

END;

CREATE TRIGGER IF NOT EXISTS trg_nodes_size_au AFTER
UPDATE OF size ON nodes WHEN NEW.size != OLD.size BEGIN
UPDATE album_meta
SET
    item_size = item_size + (NEW.size - OLD.size)
WHERE
    album_id IN (
        SELECT
            album_id
        FROM
            album_items
        WHERE
            item_id = NEW.id
    );

END