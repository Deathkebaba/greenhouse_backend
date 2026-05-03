use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use greenhouse_core::data_storage_service_dto::diary_dtos::{
    get_diary::GetDiaryEntriesQueryDto, get_diary::GetDiaryResponseDto,
    get_diary_entry::DiaryEntryResponseDto, post_diary_entry::PostDiaryEntryDtoRequest,
    put_diary_entry::PutDiaryEntryDtoRequest,
};
use uuid::Uuid;

use crate::{
    AppState,
    database::diary_models::DiaryEntry,
    router::error::{Error, HttpResult},
};

pub(crate) fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_diary_entry))
        .route("/", get(get_diary))
        .route("/{id}", put(update_diary_entry))
        .route("/{id}", get(get_diary_entry))
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
    let entries = DiaryEntry::find_by_date_range(start, end, &pool).await?;
    Ok(GetDiaryResponseDto {
        entries: entries.into_iter().map(|entry| entry.into()).collect(),
    })
}
