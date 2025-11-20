CREATE TABLE IF NOT EXISTS aliases (
    node_id TEXT NOT NULL,
    file TEXT NOT NULL,
    modified INTEGER NOT NULL,
    scan_time INTEGER NOT NULL,
    PRIMARY KEY (node_id, file),
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)