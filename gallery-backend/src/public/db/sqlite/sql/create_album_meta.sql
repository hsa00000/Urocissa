CREATE TABLE IF NOT EXISTS album_metadata (
    album_id TEXT PRIMARY KEY,
    cover_id TEXT,
    title TEXT,
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE,
    FOREIGN KEY (cover_id) REFERENCES nodes (id)
);

CREATE VIEW album_meta AS
SELECT
    -- 每一個 album 的 id
    album_nodes.id AS album_id,
    -- 這個 album 底下有幾個 item（NULL 不算）
    COUNT(album_items.item_id) AS item_count,
    -- 這個 album 底下所有 item 的 size 加總，沒有 item 就是 0
    COALESCE(SUM(item_nodes.size), 0) AS item_size
FROM
    -- 先從 nodes 裡挑出 kind = 'album' 的節點
    nodes AS album_nodes
    -- 接出每個 album 的所有 item 關聯
    LEFT JOIN album_items ON album_items.album_id = album_nodes.id
    -- 再把 item_id 接回 nodes 拿 size
    LEFT JOIN nodes AS item_nodes ON item_nodes.id = album_items.item_id
WHERE
    album_nodes.kind = 'album'
GROUP BY
    album_nodes.id;