SELECT
    COUNT(*) AS item_count,
    IFNULL (SUM(nodes.size), 0) AS total_size,
    MIN(nodes.timestamp) AS min_timestamp,
    MAX(nodes.timestamp) AS max_timestamp
FROM
    album_items
    JOIN nodes ON album_items.item_id = nodes.id
WHERE
    album_items.album_id = ?