-- AUD-09: measured 100k read costs.
--   * /api/stats joined every owner track to media_assets and read the wide
--     tracks row plus the asset row: 320-480ms. A covering owner scan plus a
--     covering asset lookup make the same aggregate index-only (~26ms).
--   * the album visibility probe (EXISTS per album) cost ~50ms for 8k albums;
--     tracks(owner_id, album_id) answers it directly (~10ms).
-- Source of truth stays tracks/media_assets; these are read-path indexes only.
CREATE INDEX tracks_owner_stats ON tracks(owner_id, media_asset_id, duration_ms);
CREATE INDEX media_assets_cover ON media_assets(id, size_bytes);
CREATE INDEX tracks_owner_album_id ON tracks(owner_id, album_id);
