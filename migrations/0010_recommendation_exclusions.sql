-- Explicit owner preferences; no existing catalog/history/consent is changed.
CREATE TABLE recommendation_exclusions (
    user_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(user_id,track_id),
    FOREIGN KEY(track_id,user_id) REFERENCES tracks(id,owner_id) ON DELETE CASCADE
);
