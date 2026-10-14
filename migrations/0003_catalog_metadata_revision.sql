-- Catalog-only edits never rewrite immutable source assets or embedded file tags.
ALTER TABLE tracks ADD COLUMN metadata_revision INTEGER NOT NULL DEFAULT 0 CHECK(metadata_revision >= 0);
