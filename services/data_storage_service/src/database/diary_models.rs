use super::{
    Error, Result,
    schema::{diary_entry, diary_entry_image, diary_entry_tag},
};
use crate::Pool;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use greenhouse_core::data_storage_service_dto::diary_dtos::{
    get_diary_entry::DiaryEntryResponseDto, image_metadata::DiaryImageMetadataDto,
    query::DiaryTagFilterModeDto,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, AsChangeset, Insertable)]
#[diesel(table_name = crate::database::schema::diary_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DiaryEntryRecord {
    id: Uuid,
    entry_date: DateTime<Utc>,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiaryEntry {
    pub(crate) id: Uuid,
    pub(crate) entry_date: DateTime<Utc>,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    images: Vec<DiaryImageMetadataDto>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::database::schema::diary_entry_tag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DiaryEntryTagRecord {
    id: Uuid,
    diary_entry_id: Uuid,
    tag: String,
    normalized_tag: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::database::schema::diary_entry_image)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DiaryEntryImageRecord {
    id: Uuid,
    diary_entry_id: Uuid,
    file_name: String,
    media_type: String,
    byte_size: i64,
    storage_key: String,
    image_data: Vec<u8>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable)]
struct DiaryEntryImageMetadataRecord {
    id: Uuid,
    diary_entry_id: Uuid,
    file_name: String,
    media_type: String,
    byte_size: i64,
    _storage_key: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredDiaryImage {
    pub(crate) metadata: DiaryImageMetadataDto,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedDiaryTag {
    tag: String,
    normalized: String,
}

impl DiaryEntry {
    pub(crate) fn new(
        entry_date: DateTime<Utc>,
        title: &str,
        content: &str,
        tags: &[String],
    ) -> Self {
        let now = chrono::Utc::now();
        let normalized_tags = normalize_tags(tags);
        Self {
            id: Uuid::new_v4(),
            entry_date,
            title: String::from(title),
            content: String::from(content),
            tags: normalized_tags.into_iter().map(|tag| tag.tag).collect(),
            created_at: now,
            updated_at: now,
            images: Vec::new(),
        }
    }

    pub(crate) async fn find_by_id(id: Uuid, pool: &Pool) -> Result<Self> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;
        let record = diary_entry::table
            .filter(diary_entry::id.eq(id))
            .first(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::Find
            })?;

        let mut entries = load_entries_with_related(vec![record], &mut conn).await?;
        entries.pop().ok_or(Error::Find)
    }

    pub(crate) async fn find_by_date_range(
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        tags: &[String],
        tag_filter_mode: DiaryTagFilterModeDto,
        pool: &Pool,
    ) -> Result<Vec<Self>> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;
        let records = diary_entry::table
            .filter(
                diary_entry::entry_date
                    .ge(start)
                    .and(diary_entry::entry_date.le(end)),
            )
            .order((diary_entry::entry_date.asc(), diary_entry::id.asc()))
            .load(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::Find
            })?;

        let mut entries = load_entries_with_related(records, &mut conn).await?;
        let normalized_filters = normalize_requested_filters(tags);
        if !normalized_filters.is_empty() {
            entries.retain(|entry| entry_matches_tag_filter(entry, &normalized_filters, tag_filter_mode));
        }

        Ok(entries)
    }

    pub(crate) async fn flush(&mut self, pool: &Pool) -> Result<()> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;
        self.updated_at = chrono::Utc::now();
        self.tags = normalize_tags(&self.tags)
            .into_iter()
            .map(|tag| tag.tag)
            .collect();

        let db_entry = DiaryEntryRecord::from(&*self);
        let tag_rows = build_tag_rows(self.id, &self.tags);

        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            let db_entry = db_entry.clone();
            let tag_rows = tag_rows.clone();

            Box::pin(async move {
                diesel::insert_into(diary_entry::table)
                    .values(&db_entry)
                    .on_conflict(diary_entry::id)
                    .do_update()
                    .set(&db_entry)
                    .execute(conn)
                    .await?;

                diesel::delete(
                    diary_entry_tag::table
                        .filter(diary_entry_tag::diary_entry_id.eq(db_entry.id)),
                )
                .execute(conn)
                .await?;

                if !tag_rows.is_empty() {
                    diesel::insert_into(diary_entry_tag::table)
                        .values(&tag_rows)
                        .execute(conn)
                        .await?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Creation
        })?;

        Ok(())
    }

    pub(crate) async fn upload_image(
        entry_id: Uuid,
        file_name: &str,
        media_type: &str,
        image_data: Vec<u8>,
        pool: &Pool,
    ) -> Result<DiaryImageMetadataDto> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        ensure_entry_exists(entry_id, &mut conn).await?;

        let trimmed_file_name = file_name.trim();
        let trimmed_media_type = media_type.trim();
        let image_id = Uuid::new_v4();
        let now = Utc::now();
        let row = DiaryEntryImageRecord {
            id: image_id,
            diary_entry_id: entry_id,
            file_name: trimmed_file_name.to_string(),
            media_type: trimmed_media_type.to_string(),
            byte_size: image_data.len() as i64,
            storage_key: format!("{entry_id}/{image_id}"),
            image_data,
            created_at: now,
        };

        diesel::insert_into(diary_entry_image::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::Creation
            })?;

        Ok(row.metadata())
    }

    pub(crate) async fn download_image(
        entry_id: Uuid,
        image_id: Uuid,
        pool: &Pool,
    ) -> Result<StoredDiaryImage> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        ensure_entry_exists(entry_id, &mut conn).await?;
        let row = find_image_record(entry_id, image_id, &mut conn).await?;

        Ok(StoredDiaryImage {
            metadata: row.metadata(),
            bytes: row.image_data,
        })
    }

    pub(crate) async fn delete_image(entry_id: Uuid, image_id: Uuid, pool: &Pool) -> Result<()> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        ensure_entry_exists(entry_id, &mut conn).await?;

        let deleted_rows = diesel::delete(
            diary_entry_image::table.filter(
                diary_entry_image::diary_entry_id
                    .eq(entry_id)
                    .and(diary_entry_image::id.eq(image_id)),
            ),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Creation
        })?;

        if deleted_rows == 0 {
            return Err(Error::Find);
        }

        Ok(())
    }
}

impl From<DiaryEntry> for DiaryEntryResponseDto {
    fn from(val: DiaryEntry) -> Self {
        DiaryEntryResponseDto {
            id: val.id.to_string(),
            date: val.entry_date.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
            title: val.title,
            content: val.content,
            tags: val.tags,
            images: val.images,
            created_at: val.created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
            updated_at: val.updated_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        }
    }
}

impl From<&DiaryEntry> for DiaryEntryRecord {
    fn from(value: &DiaryEntry) -> Self {
        Self {
            id: value.id,
            entry_date: value.entry_date,
            title: value.title.clone(),
            content: value.content.clone(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl DiaryEntryImageRecord {
    fn metadata(&self) -> DiaryImageMetadataDto {
        DiaryImageMetadataDto {
            id: self.id.to_string(),
            file_name: self.file_name.clone(),
            media_type: self.media_type.clone(),
            byte_size: self.byte_size,
            uploaded_at: self.created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        }
    }
}

impl DiaryEntryImageMetadataRecord {
    fn metadata(&self) -> DiaryImageMetadataDto {
        DiaryImageMetadataDto {
            id: self.id.to_string(),
            file_name: self.file_name.clone(),
            media_type: self.media_type.clone(),
            byte_size: self.byte_size,
            uploaded_at: self.created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        }
    }
}

fn diary_image_metadata_columns() -> (
    diary_entry_image::id,
    diary_entry_image::diary_entry_id,
    diary_entry_image::file_name,
    diary_entry_image::media_type,
    diary_entry_image::byte_size,
    diary_entry_image::storage_key,
    diary_entry_image::created_at,
) {
    (
        diary_entry_image::id,
        diary_entry_image::diary_entry_id,
        diary_entry_image::file_name,
        diary_entry_image::media_type,
        diary_entry_image::byte_size,
        diary_entry_image::storage_key,
        diary_entry_image::created_at,
    )
}

fn build_tag_rows(entry_id: Uuid, tags: &[String]) -> Vec<DiaryEntryTagRecord> {
    let now = Utc::now();

    normalize_tags(tags)
        .into_iter()
        .map(|tag| DiaryEntryTagRecord {
            id: Uuid::new_v4(),
            diary_entry_id: entry_id,
            tag: tag.tag,
            normalized_tag: tag.normalized,
            created_at: now,
        })
        .collect()
}

fn normalize_tags(tags: &[String]) -> Vec<NormalizedDiaryTag> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized_tag = trimmed.to_lowercase();
        if seen.insert(normalized_tag.clone()) {
            normalized.push(NormalizedDiaryTag {
                tag: trimmed.to_string(),
                normalized: normalized_tag,
            });
        }
    }

    normalized
}

fn normalize_requested_filters(tags: &[String]) -> Vec<String> {
    normalize_tags(tags)
        .into_iter()
        .map(|tag| tag.normalized)
        .collect()
}

fn entry_matches_tag_filter(
    entry: &DiaryEntry,
    requested_filters: &[String],
    tag_filter_mode: DiaryTagFilterModeDto,
) -> bool {
    let entry_tags: HashSet<String> = normalize_tags(&entry.tags)
        .into_iter()
        .map(|tag| tag.normalized)
        .collect();

    match tag_filter_mode {
        DiaryTagFilterModeDto::Any => requested_filters
            .iter()
            .any(|tag| entry_tags.contains(tag)),
        DiaryTagFilterModeDto::All => requested_filters
            .iter()
            .all(|tag| entry_tags.contains(tag)),
    }
}

async fn load_entries_with_related(
    records: Vec<DiaryEntryRecord>,
    conn: &mut bb8::PooledConnection<
        '_,
        diesel_async::pooled_connection::AsyncDieselConnectionManager<
            diesel_async::AsyncPgConnection,
        >,
    >,
) -> Result<Vec<DiaryEntry>> {
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let entry_ids: Vec<Uuid> = records.iter().map(|record| record.id).collect();

    let tag_rows: Vec<DiaryEntryTagRecord> = diary_entry_tag::table
        .filter(diary_entry_tag::diary_entry_id.eq_any(&entry_ids))
        .order((diary_entry_tag::created_at.asc(), diary_entry_tag::id.asc()))
        .load(conn)
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Find
        })?;

    let image_rows: Vec<DiaryEntryImageMetadataRecord> = diary_entry_image::table
        .filter(diary_entry_image::diary_entry_id.eq_any(&entry_ids))
        .order((diary_entry_image::created_at.asc(), diary_entry_image::id.asc()))
        .select(diary_image_metadata_columns())
        .load(conn)
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Find
        })?;

    let mut tags_by_entry: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in tag_rows {
        tags_by_entry
            .entry(row.diary_entry_id)
            .or_default()
            .push(row.tag);
    }

    let mut images_by_entry: HashMap<Uuid, Vec<DiaryImageMetadataDto>> = HashMap::new();
    for row in image_rows {
        images_by_entry
            .entry(row.diary_entry_id)
            .or_default()
            .push(row.metadata());
    }

    Ok(records
        .into_iter()
        .map(|record| DiaryEntry {
            id: record.id,
            entry_date: record.entry_date,
            title: record.title,
            content: record.content,
            tags: tags_by_entry.remove(&record.id).unwrap_or_default(),
            created_at: record.created_at,
            updated_at: record.updated_at,
            images: images_by_entry.remove(&record.id).unwrap_or_default(),
        })
        .collect())
}

async fn ensure_entry_exists(
    entry_id: Uuid,
    conn: &mut bb8::PooledConnection<
        '_,
        diesel_async::pooled_connection::AsyncDieselConnectionManager<
            diesel_async::AsyncPgConnection,
        >,
    >,
) -> Result<()> {
    diary_entry::table
        .select(diary_entry::id)
        .filter(diary_entry::id.eq(entry_id))
        .first::<Uuid>(conn)
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Find
        })?;

    Ok(())
}

async fn find_image_record(
    entry_id: Uuid,
    image_id: Uuid,
    conn: &mut bb8::PooledConnection<
        '_,
        diesel_async::pooled_connection::AsyncDieselConnectionManager<
            diesel_async::AsyncPgConnection,
        >,
    >,
) -> Result<DiaryEntryImageRecord> {
    diary_entry_image::table
        .filter(
            diary_entry_image::diary_entry_id
                .eq(entry_id)
                .and(diary_entry_image::id.eq(image_id)),
        )
        .first(conn)
        .await
        .map_err(|e| {
            sentry::capture_error(&e);
            Error::Find
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Pool, app};
    use diesel::debug_query;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use greenhouse_core::data_storage_service_dto::diary_dtos::{
        get_diary::GetDiaryEntriesQueryDto, image_metadata::DiaryImageMetadataDto,
    };
    use reqwest::StatusCode;
    use std::net::SocketAddr;
    use testcontainers::ContainerAsync;
    use testcontainers::ImageExt;
    use testcontainers_modules::postgres::{self, Postgres};
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    struct TestApp {
        base_url: String,
        server: tokio::task::JoinHandle<()>,
        container: ContainerAsync<Postgres>,
    }

    impl TestApp {
        async fn shutdown(self) {
            self.server.abort();
            self.container.stop().await.unwrap();
        }
    }

    #[test]
    fn test_new_diary_entry() {
        let entry_date = chrono::Utc::now();
        let title = "Test Title";
        let content = "Test Content";

        let entry = DiaryEntry::new(
            entry_date,
            title,
            content,
            &[String::from("  Harvest  "), String::from("harvest")],
        );

        assert_eq!(entry.entry_date, entry_date);
        assert_eq!(entry.title, title);
        assert_eq!(entry.content, content);
        assert_eq!(entry.tags, vec![String::from("Harvest")]);
        assert_eq!(entry.created_at, entry.updated_at);
    }

    #[test]
    fn check_for_id_collision() {
        let entry_date = chrono::Utc::now();
        let title = "Test Title";
        let content = "Test Content";

        let entry1 = DiaryEntry::new(entry_date, title, content, &[]);
        let entry2 = DiaryEntry::new(entry_date, title, content, &[]);

        assert_ne!(entry1.id, entry2.id);
    }

    #[test]
    fn check_for_created_at_and_updated_at() {
        let entry_date = chrono::Utc::now();
        let title = "Test Title";
        let content = "Test Content";
        let entry = DiaryEntry::new(entry_date, title, content, &[]);

        assert_eq!(entry.created_at, entry.updated_at);
    }

    #[test]
    fn test_into_diary_entry_response_dto() {
        let entry_date = chrono::Utc::now();
        let title = "Test Title";
        let content = "Test Content";
        let created_at = chrono::Utc::now();
        let updated_at = chrono::Utc::now();
        let entry = DiaryEntry {
            id: Uuid::new_v4(),
            entry_date,
            title: String::from(title),
            content: String::from(content),
            tags: vec![String::from("Harvest")],
            created_at,
            updated_at,
            images: vec![DiaryImageMetadataDto {
                id: Uuid::new_v4().to_string(),
                file_name: String::from("leaf.png"),
                media_type: String::from("image/png"),
                byte_size: 4,
                uploaded_at: created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
            }],
        };

        let response: DiaryEntryResponseDto = entry.into();
        assert_ne!(response.id, "");
        assert_eq!(
            response.date,
            entry_date.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()
        );
        assert_eq!(response.title, title);
        assert_eq!(response.content, content);
        assert_eq!(response.tags, vec![String::from("Harvest")]);
        assert_eq!(response.images.len(), 1);
        assert_eq!(
            response.created_at,
            created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()
        );
        assert_eq!(
            response.updated_at,
            updated_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()
        );
    }

    #[test]
    fn filters_tags_case_insensitively_for_any_and_all_modes() {
        let entry = DiaryEntry {
            id: Uuid::new_v4(),
            entry_date: chrono::Utc::now(),
            title: String::from("Tomatoes"),
            content: String::from("Harvested"),
            tags: vec![String::from("Harvest"), String::from("Tomatoes")],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            images: Vec::new(),
        };

        assert!(entry_matches_tag_filter(
            &entry,
            &[String::from("harvest")],
            DiaryTagFilterModeDto::Any,
        ));
        assert!(entry_matches_tag_filter(
            &entry,
            &[String::from("harvest"), String::from("tomatoes")],
            DiaryTagFilterModeDto::All,
        ));
        assert!(!entry_matches_tag_filter(
            &entry,
            &[String::from("harvest"), String::from("missing")],
            DiaryTagFilterModeDto::All,
        ));
    }

    #[test]
    fn diary_metadata_query_does_not_select_image_bytes() {
        let entry_id = Uuid::new_v4();
        let query = diary_entry_image::table
            .filter(diary_entry_image::diary_entry_id.eq_any(vec![entry_id]))
            .order((diary_entry_image::created_at.asc(), diary_entry_image::id.asc()))
            .select(diary_image_metadata_columns());

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();

        assert!(sql.contains("storage_key"));
        assert!(!sql.contains("image_data"));
    }

    #[tokio::test]
    async fn persists_tags_filters_entries_and_round_trips_images() {
        let test_app = spawn_test_app().await;
        let client = reqwest::Client::new();

        let created = client
            .post(format!("{}/diary/", test_app.base_url))
            .json(&serde_json::json!({
                "date": "2026-05-03T10:00:00Z",
                "title": "Tomatoes",
                "content": "Pruned and watered",
                "tags": ["Harvest", " Tomatoes "]
            }))
            .send()
            .await
            .unwrap()
            .json::<DiaryEntryResponseDto>()
            .await
            .unwrap();

        assert_eq!(created.tags, vec![String::from("Harvest"), String::from("Tomatoes")]);
        assert!(created.images.is_empty());

        client
            .post(format!("{}/diary/", test_app.base_url))
            .json(&serde_json::json!({
                "date": "2026-05-03T11:00:00Z",
                "title": "Cucumbers",
                "content": "Checked growth",
                "tags": ["Inspection"]
            }))
            .send()
            .await
            .unwrap();

        let any_filter = client
            .get(format!("{}/diary/", test_app.base_url))
            .query(&GetDiaryEntriesQueryDto {
                start: String::from("2026-05-03T00:00:00Z"),
                end: String::from("2026-05-04T00:00:00Z"),
                tags: vec![String::from("tomatoes")],
                tag_filter_mode: DiaryTagFilterModeDto::Any,
            })
            .send()
            .await
            .unwrap()
            .json::<greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::GetDiaryResponseDto>()
            .await
            .unwrap();
        assert_eq!(any_filter.entries.len(), 1);
        assert_eq!(any_filter.entries[0].id, created.id);

        let all_filter = client
            .get(format!("{}/diary/", test_app.base_url))
            .query(&GetDiaryEntriesQueryDto {
                start: String::from("2026-05-03T00:00:00Z"),
                end: String::from("2026-05-04T00:00:00Z"),
                tags: vec![String::from("harvest"), String::from("tomatoes")],
                tag_filter_mode: DiaryTagFilterModeDto::All,
            })
            .send()
            .await
            .unwrap()
            .json::<greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::GetDiaryResponseDto>()
            .await
            .unwrap();
        assert_eq!(all_filter.entries.len(), 1);
        assert_eq!(all_filter.entries[0].id, created.id);

        let image_bytes = vec![137, 80, 78, 71];
        let uploaded = client
            .post(format!("{}/diary/{}/images", test_app.base_url, created.id))
            .header("x-file-name", "leaf.png")
            .header(reqwest::header::CONTENT_TYPE, "image/png")
            .body(image_bytes.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(uploaded.status(), StatusCode::OK);
        let uploaded_metadata = uploaded.json::<DiaryImageMetadataDto>().await.unwrap();
        assert_eq!(uploaded_metadata.file_name, "leaf.png");
        assert_eq!(uploaded_metadata.media_type, "image/png");
        assert_eq!(uploaded_metadata.byte_size, image_bytes.len() as i64);

        let fetched_entry = client
            .get(format!("{}/diary/{}", test_app.base_url, created.id))
            .send()
            .await
            .unwrap()
            .json::<DiaryEntryResponseDto>()
            .await
            .unwrap();
        assert_eq!(fetched_entry.images, vec![uploaded_metadata.clone()]);

        let downloaded = client
            .get(format!(
                "{}/diary/{}/images/{}",
                test_app.base_url, created.id, uploaded_metadata.id
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(downloaded.status(), StatusCode::OK);
        assert_eq!(
            downloaded.headers()[reqwest::header::CONTENT_TYPE],
            reqwest::header::HeaderValue::from_static("image/png")
        );
        assert_eq!(downloaded.bytes().await.unwrap().to_vec(), image_bytes);

        let deleted = client
            .delete(format!(
                "{}/diary/{}/images/{}",
                test_app.base_url, created.id, uploaded_metadata.id
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

        let after_delete = client
            .get(format!("{}/diary/{}", test_app.base_url, created.id))
            .send()
            .await
            .unwrap()
            .json::<DiaryEntryResponseDto>()
            .await
            .unwrap();
        assert!(after_delete.images.is_empty());

        test_app.shutdown().await;
    }

    #[tokio::test]
    async fn image_routes_return_not_found_for_missing_resources() {
        let test_app = spawn_test_app().await;
        let client = reqwest::Client::new();
        let entry_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();

        let missing_upload = client
            .post(format!("{}/diary/{entry_id}/images", test_app.base_url))
            .header("x-file-name", "missing.png")
            .header(reqwest::header::CONTENT_TYPE, "image/png")
            .body(vec![1, 2, 3])
            .send()
            .await
            .unwrap();
        assert_eq!(missing_upload.status(), StatusCode::NOT_FOUND);

        let missing_download = client
            .get(format!("{}/diary/{entry_id}/images/{image_id}", test_app.base_url))
            .send()
            .await
            .unwrap();
        assert_eq!(missing_download.status(), StatusCode::NOT_FOUND);

        let missing_delete = client
            .delete(format!("{}/diary/{entry_id}/images/{image_id}", test_app.base_url))
            .send()
            .await
            .unwrap();
        assert_eq!(missing_delete.status(), StatusCode::NOT_FOUND);

        test_app.shutdown().await;
    }

    async fn spawn_test_app() -> TestApp {
        let container = postgres::Postgres::default()
            .with_db_name("data")
            .with_tag("latest")
            .start()
            .await
            .unwrap();
        let db_port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgres://postgres:postgres@localhost:{db_port}/data");

        let config = Config {
            service_port: 0,
            database_url: database_url.clone(),
            sentry_url: String::new(),
            environment: String::from("test"),
        };
        let pool = Pool::builder()
            .build(AsyncDieselConnectionManager::new(database_url))
            .await
            .unwrap();
        let app = app(config, pool);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        TestApp {
            base_url: format!("http://{address}"),
            server,
            container,
        }
    }
}
