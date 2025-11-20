CREATE TABLE IF NOT EXISTS extensions (
    node_id TEXT PRIMARY KEY,
    ext TEXT,
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)