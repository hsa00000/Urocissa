SELECT
    n.id,
    n.size,
    n.width,
    n.height,
    e.ext,
    n.pending,
    n.thumbhash,
    n.phash,
    n.exif,
    n.alias
FROM
    nodes n
    LEFT JOIN extensions e ON n.id = e.node_id
WHERE
    n.kind IN ('image', 'video')