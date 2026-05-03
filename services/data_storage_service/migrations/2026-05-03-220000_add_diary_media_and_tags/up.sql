CREATE TABLE diary_entry_tag (
    id UUID PRIMARY KEY,
    diary_entry_id UUID NOT NULL REFERENCES diary_entry(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    normalized_tag TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT diary_entry_tag_non_empty CHECK (btrim(tag) <> ''),
    CONSTRAINT diary_entry_tag_normalized_non_empty CHECK (btrim(normalized_tag) <> '')
);

CREATE UNIQUE INDEX diary_entry_tag_entry_normalized_idx
    ON diary_entry_tag (diary_entry_id, normalized_tag);

CREATE INDEX diary_entry_tag_normalized_idx
    ON diary_entry_tag (normalized_tag);

CREATE TABLE diary_entry_image (
    id UUID PRIMARY KEY,
    diary_entry_id UUID NOT NULL REFERENCES diary_entry(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    byte_size BIGINT NOT NULL,
    storage_key TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT diary_entry_image_file_name_non_empty CHECK (btrim(file_name) <> ''),
    CONSTRAINT diary_entry_image_media_type_non_empty CHECK (btrim(media_type) <> ''),
    CONSTRAINT diary_entry_image_storage_key_non_empty CHECK (btrim(storage_key) <> ''),
    CONSTRAINT diary_entry_image_byte_size_non_negative CHECK (byte_size >= 0)
);

CREATE INDEX diary_entry_image_entry_created_idx
    ON diary_entry_image (diary_entry_id, created_at, id);