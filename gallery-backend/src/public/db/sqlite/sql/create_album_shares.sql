CREATE TABLE IF NOT EXISTS album_shares (
    album_id TEXT,
    share_key TEXT,
    share_value TEXT,
    PRIMARY KEY (album_id, share_key),
    FOREIGN KEY (album_id) REFERENCES album_meta (album_id) ON DELETE CASCADE
)