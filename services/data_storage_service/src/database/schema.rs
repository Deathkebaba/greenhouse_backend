// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "severity"))]
    pub struct Severity;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Severity;

    alert (id) {
        id -> Uuid,
        severity -> Severity,
        identifier -> Text,
        value -> Text,
        note -> Nullable<Text>,
        created_at -> Timestamptz,
        datasource_id -> Uuid,
    }
}

diesel::table! {
    diary_entry (id) {
        id -> Uuid,
        entry_date -> Timestamptz,
        title -> Text,
        content -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    diary_entry_image (id) {
        id -> Uuid,
        diary_entry_id -> Uuid,
        file_name -> Text,
        media_type -> Text,
        byte_size -> Int8,
        storage_key -> Text,
        image_data -> Bytea,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    diary_entry_tag (id) {
        id -> Uuid,
        diary_entry_id -> Uuid,
        tag -> Text,
        normalized_tag -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(diary_entry_image -> diary_entry (diary_entry_id));
diesel::joinable!(diary_entry_tag -> diary_entry (diary_entry_id));

diesel::allow_tables_to_appear_in_same_query!(
    alert,
    diary_entry,
    diary_entry_image,
    diary_entry_tag,
);
