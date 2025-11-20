CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('image', 'video', 'album')),
    created_time INTEGER NOT NULL,
    pending BOOLEAN NOT NULL DEFAULT 0,
    width INTEGER NOT NULL CHECK (width > 0),
    height INTEGER NOT NULL CHECK (height > 0),
    size INTEGER NOT NULL,
    timestamp INTEGER
)