CREATE TABLE IF NOT EXISTS aliases (
    node_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    modified_time INTEGER NOT NULL,
    scan_time INTEGER NOT NULL,
    PRIMARY KEY (node_id, file_path),
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)