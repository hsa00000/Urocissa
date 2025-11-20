CREATE TABLE IF NOT EXISTS album_items (
    album_id TEXT,
    item_id TEXT,
    PRIMARY KEY (album_id, item_id),
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES nodes (id) ON DELETE CASCADE
);