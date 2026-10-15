CREATE INDEX tracks_owner_genre_filter ON tracks(owner_id,trim(genre) COLLATE NOCASE,id);
CREATE INDEX tracks_owner_codec_filter ON tracks(owner_id,codec COLLATE NOCASE,id);
CREATE INDEX tracks_owner_year_filter ON tracks(owner_id,year,id);
CREATE INDEX tracks_owner_title_order ON tracks(owner_id,title COLLATE NOCASE,id);
CREATE INDEX tracks_owner_newest_order ON tracks(owner_id,added_at DESC,id DESC);
