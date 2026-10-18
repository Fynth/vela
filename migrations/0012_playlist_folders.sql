CREATE TABLE playlist_folders (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 120),
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE UNIQUE INDEX playlist_folders_owner_name ON playlist_folders(owner_id,name COLLATE NOCASE);
ALTER TABLE playlists ADD COLUMN folder_id TEXT REFERENCES playlist_folders(id) ON DELETE SET NULL;
CREATE INDEX playlists_owner_folder ON playlists(owner_id,folder_id);
