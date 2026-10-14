-- Persist owner-scoped provisional catalog identities; never infer recording identity.
CREATE TABLE artists (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 name TEXT NOT NULL COLLATE NOCASE CHECK(length(name)>0),
 UNIQUE(owner_id,name), UNIQUE(owner_id,id)
);
CREATE TABLE albums (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 title TEXT NOT NULL COLLATE NOCASE CHECK(length(title)>0),
 artist_id TEXT NOT NULL, release_year INTEGER NOT NULL DEFAULT 0 CHECK(release_year>=0),
 UNIQUE(owner_id,title,artist_id,release_year), UNIQUE(owner_id,id),
 FOREIGN KEY(owner_id,artist_id) REFERENCES artists(owner_id,id)
);
ALTER TABLE tracks ADD COLUMN artist_id TEXT REFERENCES artists(id);
ALTER TABLE tracks ADD COLUMN album_id TEXT REFERENCES albums(id);
CREATE INDEX tracks_catalog_artist ON tracks(owner_id,artist_id,id);
CREATE INDEX tracks_catalog_album ON tracks(owner_id,album_id,disc_number,track_number,id);
CREATE INDEX albums_catalog_artist ON albums(owner_id,artist_id,id);

-- Association-only updates must not touch FTS, including during INSERT triggers.
DROP TRIGGER tracks_au;
CREATE TRIGGER tracks_au AFTER UPDATE OF title,artist,album,album_artist,genre ON tracks BEGIN
 INSERT INTO tracks_fts(tracks_fts,rowid,title,artist,album,album_artist,genre) VALUES('delete',old.rowid,old.title,old.artist,old.album,old.album_artist,old.genre);
 INSERT INTO tracks_fts(rowid,title,artist,album,album_artist,genre) VALUES(new.rowid,new.title,new.artist,new.album,new.album_artist,new.genre);
END;

-- Existing names remain in tracks for portability. SQLite NOCASE folds ASCII only.
-- Different known release years remain distinct; unknown years are provisional.
INSERT INTO artists(id,owner_id,name)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),owner_id,COALESCE(NULLIF(trim(artist),''),'Unknown artist')
 FROM tracks GROUP BY owner_id,COALESCE(NULLIF(trim(artist),''),'Unknown artist') COLLATE NOCASE;
INSERT INTO artists(id,owner_id,name)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),owner_id,COALESCE(NULLIF(trim(album_artist),''),NULLIF(trim(artist),''),'Unknown artist')
 FROM tracks WHERE trim(album)<>''
 GROUP BY owner_id,COALESCE(NULLIF(trim(album_artist),''),NULLIF(trim(artist),''),'Unknown artist') COLLATE NOCASE
 ON CONFLICT(owner_id,name) DO NOTHING;
INSERT INTO albums(id,owner_id,title,artist_id,release_year)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),t.owner_id,trim(t.album),a.id,max(COALESCE(t.year,0),0)
 FROM tracks t JOIN artists a ON a.owner_id=t.owner_id
 AND a.name=COALESCE(NULLIF(trim(t.album_artist),''),NULLIF(trim(t.artist),''),'Unknown artist') COLLATE NOCASE
 WHERE trim(t.album)<>'' GROUP BY t.owner_id,trim(t.album) COLLATE NOCASE,a.id,max(COALESCE(t.year,0),0);
UPDATE tracks SET
 artist_id=(SELECT id FROM artists WHERE owner_id=tracks.owner_id AND name=COALESCE(NULLIF(trim(tracks.artist),''),'Unknown artist') COLLATE NOCASE),
 album_id=(SELECT b.id FROM albums b JOIN artists a ON a.id=b.artist_id
 WHERE b.owner_id=tracks.owner_id AND b.title=trim(tracks.album) COLLATE NOCASE
 AND a.name=COALESCE(NULLIF(trim(tracks.album_artist),''),NULLIF(trim(tracks.artist),''),'Unknown artist') COLLATE NOCASE
 AND b.release_year=max(COALESCE(tracks.year,0),0));

CREATE TRIGGER tracks_catalog_insert AFTER INSERT ON tracks BEGIN
 INSERT INTO artists(id,owner_id,name) VALUES(lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,COALESCE(NULLIF(trim(new.artist),''),'Unknown artist'))
 ON CONFLICT(owner_id,name) DO NOTHING;
 INSERT INTO artists(id,owner_id,name)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') WHERE trim(new.album)<>''
 ON CONFLICT(owner_id,name) DO NOTHING;
 INSERT INTO albums(id,owner_id,title,artist_id,release_year)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,trim(new.album),a.id,max(COALESCE(new.year,0),0)
 FROM artists a WHERE a.owner_id=new.owner_id
 AND a.name=COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE AND trim(new.album)<>''
 ON CONFLICT(owner_id,title,artist_id,release_year) DO NOTHING;
 UPDATE tracks SET
 artist_id=(SELECT id FROM artists WHERE owner_id=new.owner_id AND name=COALESCE(NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE),
 album_id=(SELECT b.id FROM albums b JOIN artists a ON a.id=b.artist_id
 WHERE b.owner_id=new.owner_id AND b.title=trim(new.album) COLLATE NOCASE
 AND a.name=COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE
 AND b.release_year=max(COALESCE(new.year,0),0)) WHERE id=new.id;
END;
CREATE TRIGGER tracks_catalog_update AFTER UPDATE OF artist,album,album_artist,year ON tracks BEGIN
 INSERT INTO artists(id,owner_id,name) VALUES(lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,COALESCE(NULLIF(trim(new.artist),''),'Unknown artist'))
 ON CONFLICT(owner_id,name) DO NOTHING;
 INSERT INTO artists(id,owner_id,name)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') WHERE trim(new.album)<>''
 ON CONFLICT(owner_id,name) DO NOTHING;
 INSERT INTO albums(id,owner_id,title,artist_id,release_year)
 SELECT lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-4'||substr(lower(hex(randomblob(2))),2)||'-a'||substr(lower(hex(randomblob(2))),2)||'-'||lower(hex(randomblob(6))),new.owner_id,trim(new.album),a.id,max(COALESCE(new.year,0),0)
 FROM artists a WHERE a.owner_id=new.owner_id
 AND a.name=COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE AND trim(new.album)<>''
 ON CONFLICT(owner_id,title,artist_id,release_year) DO NOTHING;
 UPDATE tracks SET
 artist_id=(SELECT id FROM artists WHERE owner_id=new.owner_id AND name=COALESCE(NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE),
 album_id=(SELECT b.id FROM albums b JOIN artists a ON a.id=b.artist_id
 WHERE b.owner_id=new.owner_id AND b.title=trim(new.album) COLLATE NOCASE
 AND a.name=COALESCE(NULLIF(trim(new.album_artist),''),NULLIF(trim(new.artist),''),'Unknown artist') COLLATE NOCASE
 AND b.release_year=max(COALESCE(new.year,0),0)) WHERE id=new.id;
END;
-- Direct relationship rewrites may never attach someone else's catalog identity.
CREATE TRIGGER tracks_catalog_owner BEFORE UPDATE OF artist_id,album_id,owner_id ON tracks
 WHEN (new.artist_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM artists WHERE id=new.artist_id AND owner_id=new.owner_id))
 OR (new.album_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM albums WHERE id=new.album_id AND owner_id=new.owner_id))
 BEGIN SELECT RAISE(ABORT,'catalog owner mismatch'); END;
CREATE TRIGGER tracks_catalog_owner_insert BEFORE INSERT ON tracks
 WHEN (new.artist_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM artists WHERE id=new.artist_id AND owner_id=new.owner_id))
 OR (new.album_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM albums WHERE id=new.album_id AND owner_id=new.owner_id))
 BEGIN SELECT RAISE(ABORT,'catalog owner mismatch'); END;
