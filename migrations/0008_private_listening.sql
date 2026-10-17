-- Additive only: no existing catalog/media/playlist semantics change.
CREATE TABLE listening_preferences (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)),
    retention_days INTEGER NOT NULL DEFAULT 90 CHECK(retention_days BETWEEN 7 AND 365),
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
    updated_at INTEGER NOT NULL
);
CREATE TABLE listening_devices (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)),
    PRIMARY KEY(user_id,device_id)
);
CREATE UNIQUE INDEX tracks_listening_owner_key ON tracks(id,owner_id);
CREATE TABLE listening_sessions (
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 1 AND sequence <= 2147483647),
    listened_ms INTEGER NOT NULL CHECK(listened_ms BETWEEN 0 AND 86400000),
    position_ms INTEGER NOT NULL CHECK(position_ms BETWEEN 0 AND 604800000),
    started_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    qualified INTEGER NOT NULL CHECK(qualified IN (0,1)),
    ended INTEGER NOT NULL CHECK(ended IN (0,1)),
    PRIMARY KEY(user_id,session_id),
    FOREIGN KEY(user_id,device_id) REFERENCES listening_devices(user_id,device_id) ON DELETE CASCADE,
    FOREIGN KEY(track_id,user_id) REFERENCES tracks(id,owner_id) ON DELETE CASCADE
);
CREATE INDEX listening_owner_recent ON listening_sessions(user_id,qualified,updated_at DESC,session_id DESC);
CREATE INDEX listening_retention ON listening_sessions(user_id,started_at);
