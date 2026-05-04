use crate::AppState;
use crate::diary::service;
use crate::helper::error::HttpResult;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use greenhouse_core::data_storage_service_dto::diary_dtos::endpoints::{
    IMAGE_DELETE, IMAGE_DOWNLOAD, IMAGE_UPLOAD,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::{
    GetDiaryEntriesQueryDto, GetDiaryResponseDto,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary_entry::DiaryEntryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::image_metadata::DiaryImageMetadataDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::post_diary_entry::PostDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::put_diary_entry::PutDiaryEntryDtoRequest;
use uuid::Uuid;

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
pub(crate) async fn create_diary_entry(
    State(AppState { config }): State<AppState>,
    Json(entry): Json<PostDiaryEntryDtoRequest>,
) -> HttpResult<()> {
    service::create_diary_entry(&config.service_addresses.data_storage_service, entry).await?;
    Ok(())
}

#[axum::debug_handler]
pub(crate) async fn update_diary_entry(
    State(AppState { config }): State<AppState>,
    Path(id): Path<Uuid>,
    Json(update): Json<PutDiaryEntryDtoRequest>,
) -> HttpResult<()> {
    service::update_diary_entry(&config.service_addresses.data_storage_service, id, update).await?;
    Ok(())
}

#[axum::debug_handler]
pub(crate) async fn get_diary_entry(
    State(AppState { config }): State<AppState>,
    Path(id): Path<Uuid>,
) -> HttpResult<DiaryEntryResponseDto> {
    Ok(service::get_diary_entry(&config.service_addresses.data_storage_service, id).await?)
}

#[axum::debug_handler]
pub(crate) async fn get_diary(
    State(AppState { config }): State<AppState>,
    Query(query): Query<GetDiaryEntriesQueryDto>,
) -> HttpResult<GetDiaryResponseDto> {
    Ok(service::get_diary(&config.service_addresses.data_storage_service, &query).await?)
}

#[axum::debug_handler]
pub(crate) async fn upload_diary_image(
    State(AppState { config }): State<AppState>,
    Path(entry_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> HttpResult<Json<DiaryImageMetadataDto>> {
    Ok(Json(
        service::upload_diary_image(
            &config.service_addresses.data_storage_service,
            entry_id,
            &headers,
            body,
        )
        .await?,
    ))
}

#[axum::debug_handler]
pub(crate) async fn download_diary_image(
    State(AppState { config }): State<AppState>,
    Path((entry_id, image_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<impl IntoResponse> {
    let response = service::download_diary_image(
        &config.service_addresses.data_storage_service,
        entry_id,
        image_id,
    )
    .await?;

    Ok((
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&response.media_type)
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
        )],
        response.bytes,
    ))
}

#[axum::debug_handler]
pub(crate) async fn delete_diary_image(
    State(AppState { config }): State<AppState>,
    Path((entry_id, image_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<StatusCode> {
    service::delete_diary_image(
        &config.service_addresses.data_storage_service,
        entry_id,
        image_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
