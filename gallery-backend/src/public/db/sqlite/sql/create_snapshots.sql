CREATE TABLE IF NOT EXISTS snapshots (
    timestamp INTEGER,
    idx INTEGER,
    node_id TEXT,
    PRIMARY KEY (timestamp, idx),
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)