SELECT
    n.id,
    n.title,
    n.created_time,
    n.pending,
    n.width,
    n.height,
    am.start_time,
    am.end_time,
    n.last_modified_time,
    am.cover_id,
    n.thumbhash,
    am.user_defined_metadata,
    am.item_count,
    am.item_size
FROM
    nodes n
    LEFT JOIN album_meta am ON n.id = am.album_id
WHERE
    n.id = ?
    AND n.kind = 'album'