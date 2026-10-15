-- Preserve the filtering/UUID-order prefix; raw genre makes grouped facets covering.
DROP INDEX tracks_owner_genre_filter;
CREATE INDEX tracks_owner_genre_filter ON tracks(owner_id,trim(genre) COLLATE NOCASE,id,genre);
