DROP INDEX import_jobs_owner;
CREATE INDEX import_jobs_owner ON import_jobs(owner_id,created_at DESC,id DESC);
DROP INDEX scan_jobs_owner;
CREATE INDEX scan_jobs_owner ON scan_jobs(owner_id,created_at DESC,id DESC);
CREATE INDEX import_jobs_owner_status ON import_jobs(owner_id,status,created_at DESC,id DESC);
CREATE INDEX scan_jobs_owner_status ON scan_jobs(owner_id,status,created_at DESC,id DESC);
