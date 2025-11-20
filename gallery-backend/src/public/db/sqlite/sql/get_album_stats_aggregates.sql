SELECT
    COUNT(*),
    IFNULL (SUM(nodes.size), 0),
    MIN(nodes.timestamp),
    MAX(nodes.timestamp)
FROM
    album_itemsopen
    JOIN nodes ON album_items.item_id = nodes.id
WHERE
    album_items.album_id = ?