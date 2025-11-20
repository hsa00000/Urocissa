SELECT
    nodes.id,
    nodes.title,
    nodes.created_time,
    nodes.pending,
    nodes.width,
    nodes.height,
    nodes.start_time,
    nodes.end_time,
    nodes.last_modified_time,
    album_metadata.cover_id,
    nodes.thumbhash,
    album_metadata.user_defined_metadata,
    album_meta.item_count,
    album_meta.item_size
FROM
    nodes
    LEFT JOIN album_metadata ON nodes.id = album_metadata.album_id
    LEFT JOIN album_meta ON nodes.id = album_meta.album_id
WHERE
    nodes.id = ?
    AND nodes.kind = 'album'