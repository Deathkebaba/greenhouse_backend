use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use greenhouse_core::data_storage_service_dto::diary_dtos::{
    endpoints::{IMAGE_DELETE, IMAGE_DOWNLOAD, IMAGE_UPLOAD},
    get_diary::GetDiaryEntriesQueryDto, get_diary::GetDiaryResponseDto,
    get_diary_entry::DiaryEntryResponseDto, image_metadata::DiaryImageMetadataDto,
    post_diary_entry::PostDiaryEntryDtoRequest, put_diary_entry::PutDiaryEntryDtoRequest,
};
use uuid::Uuid;

use crate::{
    AppState,
    database::diary_models::{DiaryEntry, StoredDiaryImage},
    router::error::{Error, HttpResult},
};

const FILE_NAME_HEADER: &str = "x-file-name";

pub(crate) fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_diary_entry))
        .route("/", get(get_diary))
        .route("/{id}", put(update_diary_entry))
        .route("/{id}", get(get_diary_entry))
        .route(IMAGE_UPLOAD, post(upload_diary_image))
        .route(IMAGE_DOWNLOAD, get(download_diary_image))
        .route(IMAGE_DELETE, delete(delete_diary_image))
        .with_state(state)
}

#[axum::debug_handler]
pub(crate) async fn update_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path(id): Path<Uuid>,
    Json(update): Json<PutDiaryEntryDtoRequest>,
) -> HttpResult<DiaryEntryResponseDto> {
    let mut entry = DiaryEntry::find_by_id(id, &pool).await?;
    entry.title = update.title.clone();
    entry.entry_date = update.date.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), update.date.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;

    entry.content = update.content.clone();
    entry.tags = update.tags.clone();
    entry.flush(&pool).await?;
    Ok(entry.into())
}

#[axum::debug_handler]
pub(crate) async fn create_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Json(entry): Json<PostDiaryEntryDtoRequest>,
) -> HttpResult<DiaryEntryResponseDto> {
    let mut entry = DiaryEntry::new(
        entry.date.parse::<DateTime<Utc>>().map_err(|e| {
            sentry::configure_scope(|scope| {
                let mut map = std::collections::BTreeMap::new();
                map.insert(String::from("time"), entry.date.clone().into());

                scope.set_context("time_string", sentry::protocol::Context::Other(map));
            });

            sentry::capture_error(&e);
            Error::TimeError
        })?,
        &entry.title,
        &entry.content,
        &entry.tags,
    );
    entry.flush(&pool).await?;
    Ok(entry.into())
}

#[axum::debug_handler]
pub(crate) async fn get_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path(id): Path<Uuid>,
) -> HttpResult<DiaryEntryResponseDto> {
    Ok(DiaryEntry::find_by_id(id, &pool).await?.into())
}

#[axum::debug_handler]
pub(crate) async fn get_diary(
    State(AppState { config: _, pool }): State<AppState>,
    Query(query): Query<GetDiaryEntriesQueryDto>,
) -> HttpResult<GetDiaryResponseDto> {
    let start = query.start.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), query.start.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;
    let end = query.end.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), query.end.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;
    let entries = DiaryEntry::find_by_date_range(start, end, &query.tags, query.tag_filter_mode, &pool).await?;
    Ok(GetDiaryResponseDto {
        entries: entries.into_iter().map(|entry| entry.into()).collect(),
    })
}

#[axum::debug_handler]
pub(crate) async fn upload_diary_image(
    State(AppState { config: _, pool }): State<AppState>,
    Path(entry_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> HttpResult<Json<DiaryImageMetadataDto>> {
    let file_name = required_header(&headers, FILE_NAME_HEADER)?;
    let media_type = required_header(&headers, header::CONTENT_TYPE.as_str())?;

    if body.is_empty() {
        return Err(Error::InvalidRequest(String::from("Image body must not be empty")).into());
    }

    if file_name.trim().is_empty() {
        return Err(Error::InvalidRequest(String::from("Image file name is required")).into());
    }

    if media_type.trim().is_empty() {
        return Err(Error::InvalidRequest(String::from("Image media type is required")).into());
    }

    let metadata = DiaryEntry::upload_image(entry_id, &file_name, &media_type, body.to_vec(), &pool).await?;
    Ok(Json(metadata))
}

#[axum::debug_handler]
pub(crate) async fn download_diary_image(
    State(AppState { config: _, pool }): State<AppState>,
    Path((entry_id, image_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<axum::response::Response> {
    let StoredDiaryImage { metadata, bytes } = DiaryEntry::download_image(entry_id, image_id, &pool).await?;
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(&metadata.media_type).map_err(|_| {
            Error::InvalidRequest(String::from("Stored image media type is invalid"))
        })?,
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_str(&metadata.byte_size.to_string()).map_err(|_| {
            Error::InvalidRequest(String::from("Stored image size is invalid"))
        })?,
    );
    Ok(response)
}

#[axum::debug_handler]
pub(crate) async fn delete_diary_image(
    State(AppState { config: _, pool }): State<AppState>,
    Path((entry_id, image_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<StatusCode> {
    DiaryEntry::delete_image(entry_id, image_id, &pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn required_header(headers: &HeaderMap, key: &str) -> HttpResult<String> {
    headers
        .get(key)
        .ok_or_else(|| Error::InvalidRequest(format!("Missing required header: {key}")))?
        .to_str()
        .map(|value| value.to_string())
        .map_err(|_| Error::InvalidRequest(format!("Invalid header value for {key}")).into())
}
