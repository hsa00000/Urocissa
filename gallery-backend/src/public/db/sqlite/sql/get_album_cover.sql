-- 優先返回用戶指定的 cover_id，如果沒有則返回最舊的項目
SELECT
    COALESCE(
        (
            SELECT
                cover_id
            FROM
                album_metadata
            WHERE
                album_id = ?
                AND cover_id IS NOT NULL
        ),
        (
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
        )
    ) AS cover_id