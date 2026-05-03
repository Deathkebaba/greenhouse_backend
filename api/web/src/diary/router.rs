use crate::AppState;
use crate::diary::service;
use crate::helper::error::HttpResult;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::{
    GetDiaryEntriesQueryDto, GetDiaryResponseDto,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary_entry::DiaryEntryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::post_diary_entry::PostDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::put_diary_entry::PutDiaryEntryDtoRequest;
use uuid::Uuid;

pub(crate) fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_diary_entry))
    .route("/", get(get_diary))
        .route("/{id}", put(update_diary_entry))
        .route("/{id}", get(get_diary_entry))
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
