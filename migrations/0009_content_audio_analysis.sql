-- Versioned derived cache, independent of playback and consented personal history.
CREATE TABLE audio_analysis (
    media_asset_id TEXT NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    extractor_version TEXT NOT NULL,
    requested_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    status TEXT NOT NULL CHECK(status IN ('queued','running','complete','failed')),
    features_json TEXT CHECK(features_json IS NULL OR length(features_json)<=16384),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(media_asset_id,extractor_version)
);
CREATE INDEX audio_analysis_work ON audio_analysis(extractor_version,status,created_at,media_asset_id);
CREATE INDEX audio_analysis_active ON audio_analysis(extractor_version,requested_by) WHERE status IN ('queued','running');
