use greenhouse_core::{
    data_storage_service_dto::diary_dtos::{
        endpoints, get_diary::GetDiaryEntriesQueryDto, get_diary::GetDiaryResponseDto,
        get_diary_entry::DiaryEntryResponseDto, post_diary_entry::PostDiaryEntryDtoRequest,
        put_diary_entry::PutDiaryEntryDtoRequest,
    },
    http_error::ErrorResponseBody,
};
use uuid::Uuid;

use crate::{
    diary::{Error, Result},
    helper::error::ApiError,
};

pub(crate) async fn create_diary_entry(
    base_ulr: &str,
    entry: PostDiaryEntryDtoRequest,
) -> Result<()> {
    let resp = reqwest::Client::new()
        .post(base_ulr.to_string() + endpoints::DIARY)
        .json(&entry)
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in post to service: {:?} with entry: {:?} for url {}",
                e,
                entry,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        return Ok(());
    }
    Err(Error::Api(ApiError {
        status: resp.status(),
        message: resp
            .json::<ErrorResponseBody>()
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                tracing::error!("Error in get to service: {:?}", e);
                Error::Json(e)
            })?
            .error,
    }))
}

pub(crate) async fn update_diary_entry(
    base_ulr: &str,
    id: Uuid,
    update: PutDiaryEntryDtoRequest,
) -> Result<()> {
    let resp = reqwest::Client::new()
        .put(base_ulr.to_string() + endpoints::DIARY + "/" + &id.to_string())
        .json(&update)
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in put to service: {:?} with entry: {:?} for url {}",
                e,
                update,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        return Ok(());
    }
    Err(Error::Api(ApiError {
        status: resp.status(),
        message: resp
            .json::<ErrorResponseBody>()
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                tracing::error!("Error in get to service: {:?}", e);
                Error::Json(e)
            })?
            .error,
    }))
}

pub(crate) async fn get_diary_entry(base_ulr: &str, id: Uuid) -> Result<DiaryEntryResponseDto> {
    let resp = reqwest::Client::new()
        .get(base_ulr.to_string() + endpoints::DIARY + "/" + &id.to_string())
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in get to service: {:?} with id: {:?} for url {}",
                e,
                id,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        return resp.json().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error in get to service: {:?}", e,);
            Error::Json(e)
        });
    }
    Err(Error::Api(ApiError {
        status: resp.status(),
        message: resp
            .json::<ErrorResponseBody>()
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                tracing::error!("Error in get to service: {:?}", e);
                Error::Json(e)
            })?
            .error,
    }))
}

pub(crate) async fn get_diary(
    base_ulr: &str,
    query: &GetDiaryEntriesQueryDto,
) -> Result<GetDiaryResponseDto> {
    let resp = reqwest::Client::new()
        .get(base_ulr.to_string() + endpoints::DIARY)
        .query(query)
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in get to service: {:?} with query: {:?} for url {}",
                e,
                query,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        return resp.json().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error in get to service: {:?}", e,);
            Error::Json(e)
        });
    }
    Err(Error::Api(ApiError {
        status: resp.status(),
        message: resp
            .json::<ErrorResponseBody>()
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                tracing::error!("Error in get to service: {:?}", e);
                Error::Json(e)
            })?
            .error,
    }))
}
