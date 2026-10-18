-- Disabled by default; no existing track/asset/user becomes public.
CREATE TABLE network_settings (
 id INTEGER PRIMARY KEY CHECK(id=1),
 server_id TEXT NOT NULL UNIQUE,
 enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN(0,1)),
 origin TEXT,
 name TEXT NOT NULL,
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0)
);
CREATE TABLE publications (
 id TEXT PRIMARY KEY,
 user_id TEXT NOT NULL,
 track_id TEXT NOT NULL,
 license TEXT NOT NULL CHECK(license IN('CC0-1.0','CC-BY-4.0','CC-BY-SA-4.0')),
 attribution TEXT NOT NULL CHECK(length(attribution) BETWEEN 1 AND 2000),
 rights_evidence TEXT NOT NULL CHECK(length(rights_evidence) BETWEEN 1 AND 2048),
 allow_stream INTEGER NOT NULL DEFAULT 0 CHECK(allow_stream IN(0,1)),
 allow_download INTEGER NOT NULL DEFAULT 0 CHECK(allow_download IN(0,1) AND allow_download<=allow_stream),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>=1),
 created_at INTEGER NOT NULL,
 revoked_at INTEGER,
 FOREIGN KEY(track_id,user_id) REFERENCES tracks(id,owner_id) ON DELETE CASCADE
);
CREATE INDEX publications_owner_cursor ON publications(user_id,id);
CREATE INDEX publications_public_cursor ON publications(revoked_at,id);
CREATE UNIQUE INDEX publications_one_active ON publications(user_id,track_id) WHERE revoked_at IS NULL;
