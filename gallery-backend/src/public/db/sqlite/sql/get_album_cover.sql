SELECT
    nodes.id
FROM
    album_items
    JOIN nodes ON album_items.item_id = nodes.id
WHERE
    album_items.album_id = ?
ORDER BY
    nodes.timestamp ASC
LIMIT
    1