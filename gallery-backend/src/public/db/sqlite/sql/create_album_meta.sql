CREATE TABLE IF NOT EXISTS album_metadata (
    album_id TEXT PRIMARY KEY,
    cover_id TEXT,
    user_defined_metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE,
    FOREIGN KEY (cover_id) REFERENCES nodes (id)
);

CREATE VIEW album_meta AS
SELECT
    album_nodes.id AS album_id,
    COUNT(album_items.item_id) AS item_count,
    COALESCE(SUM(item_nodes.size), 0) AS item_size
FROM
    nodes AS album_nodes
    LEFT JOIN album_items ON album_items.album_id = album_nodes.id
    LEFT JOIN nodes AS item_nodes ON item_nodes.id = album_items.item_id
WHERE
    album_nodes.kind = 'album'
GROUP BY
    album_nodes.id;