CREATE TABLE IF NOT EXISTS exif (
    node_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    value TEXT,
    PRIMARY KEY (node_id, tag),
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)