CREATE TABLE IF NOT EXISTS nodes_tags (
    node_id TEXT,
    tag TEXT,
    PRIMARY KEY (node_id, tag),
    FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
)