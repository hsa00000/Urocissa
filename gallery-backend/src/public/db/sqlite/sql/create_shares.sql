CREATE TABLE IF NOT EXISTS shares (
    url TEXT PRIMARY KEY,
    album_id TEXT NOT NULL,
    description TEXT NOT NULL,
    password TEXT,
    show_metadata BOOLEAN NOT NULL DEFAULT 1,
    show_download BOOLEAN NOT NULL DEFAULT 1,
    show_upload BOOLEAN NOT NULL DEFAULT 0,
    exp INTEGER NOT NULL,
    FOREIGN KEY (album_id) REFERENCES nodes (id) ON DELETE CASCADE
)