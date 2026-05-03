ALTER TABLE diary_entry_image
    ADD COLUMN image_data BYTEA NOT NULL DEFAULT '\x'::bytea;