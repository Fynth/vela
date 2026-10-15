CREATE TABLE scan_sources (
 id TEXT PRIMARY KEY,
 owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 120),
 root_id TEXT NOT NULL, last_scan_at INTEGER,
 UNIQUE(owner_id,root_id), UNIQUE(owner_id,id)
);
CREATE TABLE scan_jobs (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, source_id TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed','cancelled')),
 cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
 visited INTEGER NOT NULL DEFAULT 0 CHECK(visited>=0),
 imported INTEGER NOT NULL DEFAULT 0 CHECK(imported>=0),
 duplicates INTEGER NOT NULL DEFAULT 0 CHECK(duplicates>=0),
 unchanged INTEGER NOT NULL DEFAULT 0 CHECK(unchanged>=0),
 skipped INTEGER NOT NULL DEFAULT 0 CHECK(skipped>=0),
 failed_files INTEGER NOT NULL DEFAULT 0 CHECK(failed_files>=0),
 error TEXT, created_at INTEGER NOT NULL DEFAULT(unixepoch()),
 updated_at INTEGER NOT NULL DEFAULT(unixepoch()), finished_at INTEGER,
 FOREIGN KEY(owner_id,source_id) REFERENCES scan_sources(owner_id,id) ON DELETE CASCADE
);
CREATE INDEX scan_jobs_worker ON scan_jobs(status,created_at,id);
CREATE INDEX scan_jobs_owner ON scan_jobs(owner_id,created_at DESC,id);
CREATE UNIQUE INDEX scan_jobs_active_source ON scan_jobs(source_id) WHERE status IN ('queued','running');
CREATE TABLE scan_files (
 source_id TEXT NOT NULL REFERENCES scan_sources(id) ON DELETE CASCADE,
 path_key TEXT NOT NULL, fingerprint TEXT NOT NULL,
 track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
 updated_at INTEGER NOT NULL DEFAULT(unixepoch()),
 PRIMARY KEY(source_id,path_key)
);
CREATE TRIGGER scan_files_owner_insert BEFORE INSERT ON scan_files
 WHEN NOT EXISTS(SELECT 1 FROM scan_sources s JOIN tracks t ON t.owner_id=s.owner_id WHERE s.id=new.source_id AND t.id=new.track_id)
 BEGIN SELECT RAISE(ABORT,'scan file owner mismatch'); END;
CREATE TRIGGER scan_files_owner_update BEFORE UPDATE OF source_id,track_id ON scan_files
 WHEN NOT EXISTS(SELECT 1 FROM scan_sources s JOIN tracks t ON t.owner_id=s.owner_id WHERE s.id=new.source_id AND t.id=new.track_id)
 BEGIN SELECT RAISE(ABORT,'scan file owner mismatch'); END;
CREATE TRIGGER scan_source_owner_update BEFORE UPDATE OF owner_id ON scan_sources
 WHEN EXISTS(SELECT 1 FROM scan_files f JOIN tracks t ON t.id=f.track_id WHERE f.source_id=new.id AND t.owner_id<>new.owner_id)
 BEGIN SELECT RAISE(ABORT,'scan source owner mismatch'); END;
CREATE TRIGGER scan_track_owner_update BEFORE UPDATE OF owner_id ON tracks
 WHEN EXISTS(SELECT 1 FROM scan_files f JOIN scan_sources s ON s.id=f.source_id WHERE f.track_id=new.id AND s.owner_id<>new.owner_id)
 BEGIN SELECT RAISE(ABORT,'scan track owner mismatch'); END;
