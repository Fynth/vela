CREATE TABLE users (
 id TEXT PRIMARY KEY, username TEXT NOT NULL UNIQUE COLLATE NOCASE,
 password_hash TEXT NOT NULL, is_admin INTEGER NOT NULL DEFAULT 0 CHECK(is_admin IN (0,1)),
 created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE TABLE sessions (
 token_hash TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 csrf_token TEXT NOT NULL, expires_at INTEGER NOT NULL,
 created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE INDEX sessions_expiry ON sessions(expires_at);
CREATE TABLE media_assets (
 id TEXT PRIMARY KEY, hash TEXT NOT NULL UNIQUE, storage_path TEXT NOT NULL,
 content_type TEXT NOT NULL, size_bytes INTEGER NOT NULL CHECK(size_bytes > 0)
);
CREATE TABLE artwork (
 id TEXT PRIMARY KEY, hash TEXT NOT NULL UNIQUE, storage_path TEXT NOT NULL
);
CREATE TABLE tracks (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 media_asset_id TEXT NOT NULL REFERENCES media_assets(id),
 title TEXT NOT NULL, artist TEXT NOT NULL DEFAULT '', album TEXT NOT NULL DEFAULT '',
 album_artist TEXT NOT NULL DEFAULT '', genre TEXT NOT NULL DEFAULT '', year INTEGER,
 track_number INTEGER, disc_number INTEGER, duration_ms INTEGER NOT NULL,
 codec TEXT NOT NULL, bitrate INTEGER, sample_rate INTEGER, bit_depth INTEGER, channels INTEGER,
 artwork_id TEXT REFERENCES artwork(id), added_at INTEGER NOT NULL DEFAULT(unixepoch()),
 UNIQUE(owner_id, media_asset_id)
);
CREATE INDEX tracks_owner_cursor ON tracks(owner_id,id);
CREATE INDEX tracks_owner_album ON tracks(owner_id,album,album_artist);
CREATE INDEX tracks_owner_artist ON tracks(owner_id,artist);
CREATE VIRTUAL TABLE tracks_fts USING fts5(title,artist,album,album_artist,genre,content='tracks',content_rowid='rowid',tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER tracks_ai AFTER INSERT ON tracks BEGIN
 INSERT INTO tracks_fts(rowid,title,artist,album,album_artist,genre) VALUES(new.rowid,new.title,new.artist,new.album,new.album_artist,new.genre);
END;
CREATE TRIGGER tracks_ad AFTER DELETE ON tracks BEGIN
 INSERT INTO tracks_fts(tracks_fts,rowid,title,artist,album,album_artist,genre) VALUES('delete',old.rowid,old.title,old.artist,old.album,old.album_artist,old.genre);
END;
CREATE TRIGGER tracks_au AFTER UPDATE ON tracks BEGIN
 INSERT INTO tracks_fts(tracks_fts,rowid,title,artist,album,album_artist,genre) VALUES('delete',old.rowid,old.title,old.artist,old.album,old.album_artist,old.genre);
 INSERT INTO tracks_fts(rowid,title,artist,album,album_artist,genre) VALUES(new.rowid,new.title,new.artist,new.album,new.album_artist,new.genre);
END;
CREATE TABLE favorites (
 user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
 PRIMARY KEY(user_id,track_id)
);
CREATE TABLE import_jobs (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 url TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('queued','downloading','processing','completed','failed','cancelled')),
 downloaded_bytes INTEGER NOT NULL DEFAULT 0, total_bytes INTEGER, error TEXT,
 track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL, attempt INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL DEFAULT(unixepoch()), updated_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE INDEX import_jobs_worker ON import_jobs(status,created_at);
CREATE INDEX import_jobs_owner ON import_jobs(owner_id,created_at DESC);
CREATE TABLE playlists (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 120), revision INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE TABLE playlist_items (
 id TEXT PRIMARY KEY, playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
 track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0), UNIQUE(playlist_id,position)
);
CREATE TABLE playlist_collaborators (
 playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
 user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 can_edit INTEGER NOT NULL CHECK(can_edit IN (0,1)), PRIMARY KEY(playlist_id,user_id)
);
CREATE TABLE playlist_mutations (
 playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
 mutation_id TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 request_hash TEXT NOT NULL, result_revision INTEGER NOT NULL,
 created_at INTEGER NOT NULL DEFAULT(unixepoch()), PRIMARY KEY(playlist_id,mutation_id)
);
CREATE TABLE playlist_shares (
 id TEXT PRIMARY KEY, playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
 token_hash TEXT NOT NULL UNIQUE, expires_at INTEGER NOT NULL,
 created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
CREATE TABLE audit_events (
 id INTEGER PRIMARY KEY, actor_id TEXT REFERENCES users(id) ON DELETE SET NULL,
 action TEXT NOT NULL, object_id TEXT, created_at INTEGER NOT NULL DEFAULT(unixepoch())
);
