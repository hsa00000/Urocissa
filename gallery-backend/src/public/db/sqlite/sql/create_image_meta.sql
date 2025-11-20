CREATE TABLE IF NOT EXISTS image_meta (
    node_id TEXT PRIMARY KEY,
    thumbhash BLOB,
    phash BLOB,
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
);