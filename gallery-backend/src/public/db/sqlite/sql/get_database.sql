SELECT
    nodes.id,
    nodes.size,
    nodes.width,
    nodes.height,
    extensions.ext,
    nodes.pending,
    image_meta.thumbhash,
    image_meta.phash,
    nodes.exif,
    nodes.alias
FROM
    nodes
    LEFT JOIN extensions ON nodes.id = extensions.node_id
    LEFT JOIN image_meta ON nodes.id = image_meta.node_id
WHERE
    nodes.id = ?
    AND nodes.kind IN ('image', 'video')