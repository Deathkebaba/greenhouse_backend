use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode, header};
use greenhouse_core::{
    data_storage_service_dto::diary_dtos::{
        endpoints, get_diary::GetDiaryEntriesQueryDto, get_diary::GetDiaryResponseDto,
        get_diary_entry::DiaryEntryResponseDto, image_metadata::DiaryImageMetadataDto,
        post_diary_entry::PostDiaryEntryDtoRequest, put_diary_entry::PutDiaryEntryDtoRequest,
    },
    http_error::ErrorResponseBody,
};
use uuid::Uuid;

use crate::{
    diary::{Error, Result},
    helper::error::ApiError,
};

pub(crate) const FILE_NAME_HEADER: &str = "x-file-name";
const API_DIARY_BASE_PATH: &str = "/api/diary";

pub(crate) struct DiaryImageDownloadDto {
    pub(crate) bytes: Vec<u8>,
    pub(crate) media_type: String,
}

fn diary_service_url(base_url: &str, route: &str) -> String {
    format!("{base_url}{}{route}", endpoints::DIARY)
}

fn diary_image_route(route: &str, entry_id: &str, image_id: Option<&str>) -> String {
    let route = route.replace("{entry_id}", entry_id);

    match image_id {
        Some(image_id) => route.replace("{image_id}", image_id),
        None => route,
    }
}

fn diary_image_service_url(
    base_url: &str,
    route: &str,
    entry_id: &str,
    image_id: Option<&str>,
) -> String {
    diary_service_url(base_url, &diary_image_route(route, entry_id, image_id))
}

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
        let entry = resp.json().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error in get to service: {:?}", e,);
            Error::Json(e)
        })?;
        return Ok(with_backend_relative_download_urls(entry));
    }
    Err(service_error(resp).await)
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
        let response = resp.json().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error in get to service: {:?}", e,);
            Error::Json(e)
        })?;
        return Ok(with_backend_relative_download_urls_for_list(response));
    }
    Err(service_error(resp).await)
}

pub(crate) async fn upload_diary_image(
    base_ulr: &str,
    entry_id: Uuid,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<DiaryImageMetadataDto> {
    let file_name = required_header(headers, FILE_NAME_HEADER)?;
    let media_type = required_header(headers, header::CONTENT_TYPE.as_str())?;
    let entry_id = entry_id.to_string();
    let resp = reqwest::Client::new()
        .post(diary_image_service_url(
            base_ulr,
            endpoints::IMAGE_UPLOAD,
            &entry_id,
            None,
        ))
        .header(FILE_NAME_HEADER, file_name.clone())
        .header(header::CONTENT_TYPE, media_type.clone())
        .body(body)
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in post image to service: {:?} with entry_id: {:?} for url {}",
                e,
                entry_id,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        let metadata = resp.json().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error in post image to service: {:?}", e);
            Error::Json(e)
        })?;
        return Ok(with_backend_relative_download_url(&entry_id, metadata));
    }
    Err(service_error(resp).await)
}

pub(crate) async fn download_diary_image(
    base_ulr: &str,
    entry_id: Uuid,
    image_id: Uuid,
) -> Result<DiaryImageDownloadDto> {
    let entry_id = entry_id.to_string();
    let image_id = image_id.to_string();
    let resp = reqwest::Client::new()
        .get(diary_image_service_url(
            base_ulr,
            endpoints::IMAGE_DOWNLOAD,
            &entry_id,
            Some(&image_id),
        ))
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in get image from service: {:?} with entry_id: {:?}, image_id: {:?} for url {}",
                e,
                entry_id,
                image_id,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        let media_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| Error::Api(ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: String::from("Diary image response is missing content type"),
            }))?;
        let bytes = resp.bytes().await.map_err(|e| {
            sentry::capture_error(&e);
            tracing::error!("Error reading image bytes from service: {:?}", e);
            Error::Request(e)
        })?;
        return Ok(DiaryImageDownloadDto {
            bytes: bytes.to_vec(),
            media_type,
        });
    }
    Err(service_error(resp).await)
}

pub(crate) async fn delete_diary_image(base_ulr: &str, entry_id: Uuid, image_id: Uuid) -> Result<()> {
    let entry_id = entry_id.to_string();
    let image_id = image_id.to_string();
    let resp = reqwest::Client::new()
        .delete(diary_image_service_url(
            base_ulr,
            endpoints::IMAGE_DELETE,
            &entry_id,
            Some(&image_id),
        ))
        .send()
        .await
        .map_err(|e| {
            sentry::capture_error(&e);

            tracing::error!(
                "Error in delete image to service: {:?} with entry_id: {:?}, image_id: {:?} for url {}",
                e,
                entry_id,
                image_id,
                base_ulr
            );

            Error::Request(e)
        })?;
    if resp.status().is_success() {
        return Ok(());
    }
    Err(service_error(resp).await)
}

fn required_header(headers: &HeaderMap, key: &str) -> Result<String> {
    headers
        .get(key)
        .ok_or_else(|| Error::Api(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: format!("Missing required header: {key}"),
        }))?
        .to_str()
        .map(str::to_string)
        .map_err(|_| Error::Api(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: format!("Invalid header value for {key}"),
        }))
}

async fn service_error(resp: reqwest::Response) -> Error {
    Error::Api(ApiError {
        status: resp.status(),
        message: resp
            .json::<ErrorResponseBody>()
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                tracing::error!("Error in get to service: {:?}", e);
                Error::Json(e)
            })
            .map(|body| body.error)
            .unwrap_or_else(|error| error.to_string()),
    })
}

fn with_backend_relative_download_urls_for_list(
    mut response: GetDiaryResponseDto,
) -> GetDiaryResponseDto {
    response.entries = response
        .entries
        .into_iter()
        .map(with_backend_relative_download_urls)
        .collect();
    response
}

fn with_backend_relative_download_urls(mut entry: DiaryEntryResponseDto) -> DiaryEntryResponseDto {
    let entry_id = entry.id.clone();
    entry.images = entry
        .images
        .into_iter()
        .map(|image| with_backend_relative_download_url_from_ids(&entry_id, image))
        .collect();
    entry
}

fn with_backend_relative_download_url(
    entry_id: &str,
    image: DiaryImageMetadataDto,
) -> DiaryImageMetadataDto {
    with_backend_relative_download_url_from_ids(entry_id, image)
}

fn with_backend_relative_download_url_from_ids(
    entry_id: &str,
    mut image: DiaryImageMetadataDto,
) -> DiaryImageMetadataDto {
    image.download_url = format!(
        "{API_DIARY_BASE_PATH}{}",
        diary_image_route(endpoints::IMAGE_DOWNLOAD, entry_id, Some(&image.id))
    );
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        body::Body,
        extract::{Path, Query, State},
        response::IntoResponse,
        routing::{delete, get, post},
    };
    use greenhouse_core::data_storage_service_dto::diary_dtos::query::DiaryTagFilterModeDto;
    use serde::Deserialize;
    use std::sync::{Arc, Mutex};
    use tokio::{net::TcpListener, task::JoinHandle};
    use uuid::uuid;

    #[derive(Clone)]
    struct TestState {
        captured_query: Arc<Mutex<Option<GetDiaryEntriesQueryDto>>>,
        captured_upload: Arc<Mutex<Option<CapturedUpload>>>,
        captured_download: Arc<Mutex<Option<(String, String)>>>,
        captured_delete: Arc<Mutex<Option<(String, String)>>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct CapturedUpload {
        entry_id: String,
        file_name: String,
        media_type: String,
        body: Vec<u8>,
    }

    struct TestServer {
        base_url: String,
        state: TestState,
        task: JoinHandle<()>,
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    #[derive(Deserialize)]
    struct DownloadParams {
        entry_id: String,
        image_id: String,
    }

    async fn spawn_test_server() -> TestServer {
        let state = TestState {
            captured_query: Arc::new(Mutex::new(None)),
            captured_upload: Arc::new(Mutex::new(None)),
            captured_download: Arc::new(Mutex::new(None)),
            captured_delete: Arc::new(Mutex::new(None)),
        };

        let app = Router::new()
            .route(endpoints::DIARY, get(test_get_diary))
            .route(
                &format!("{}{}", endpoints::DIARY, endpoints::IMAGE_UPLOAD),
                post(test_upload_diary_image),
            )
            .route(
                &format!("{}{}", endpoints::DIARY, endpoints::IMAGE_DOWNLOAD),
                get(test_download_diary_image),
            )
            .route(
                &format!("{}{}", endpoints::DIARY, endpoints::IMAGE_DELETE),
                delete(test_delete_diary_image),
            )
            .with_state(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        TestServer {
            base_url: format!("http://{address}"),
            state,
            task,
        }
    }

    async fn test_get_diary(
        State(state): State<TestState>,
        Query(query): Query<GetDiaryEntriesQueryDto>,
    ) -> Json<GetDiaryResponseDto> {
        state.captured_query.lock().unwrap().replace(query);

        Json(GetDiaryResponseDto {
            entries: vec![DiaryEntryResponseDto {
                id: String::from("entry-1"),
                date: String::from("2026-05-01T00:00:00Z"),
                title: String::from("Title"),
                content: String::from("Content"),
                tags: vec![String::from("Tomatoes")],
                images: vec![sample_image()],
                created_at: String::from("2026-05-01T00:00:00Z"),
                updated_at: String::from("2026-05-01T00:00:00Z"),
            }],
        })
    }

    async fn test_upload_diary_image(
        State(state): State<TestState>,
        Path(entry_id): Path<String>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Json<DiaryImageMetadataDto> {
        state.captured_upload.lock().unwrap().replace(CapturedUpload {
            entry_id,
            file_name: headers.get(FILE_NAME_HEADER).unwrap().to_str().unwrap().to_string(),
            media_type: headers
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            body: body.to_vec(),
        });

        Json(sample_image())
    }

    async fn test_download_diary_image(
        State(state): State<TestState>,
        Path(params): Path<DownloadParams>,
    ) -> impl IntoResponse {
        state
            .captured_download
            .lock()
            .unwrap()
            .replace((params.entry_id, params.image_id));

        (
            [(header::CONTENT_TYPE, "image/jpeg")],
            Body::from(vec![9_u8, 8, 7]),
        )
    }

    async fn test_delete_diary_image(
        State(state): State<TestState>,
        Path(params): Path<DownloadParams>,
    ) -> StatusCode {
        state
            .captured_delete
            .lock()
            .unwrap()
            .replace((params.entry_id, params.image_id));

        StatusCode::NO_CONTENT
    }

    fn sample_image() -> DiaryImageMetadataDto {
        DiaryImageMetadataDto {
            id: String::from("img-1"),
            file_name: String::from("leaf.png"),
            media_type: String::from("image/png"),
            byte_size: 4,
            uploaded_at: String::from("2026-05-01T00:00:00Z"),
            download_url: String::new(),
        }
    }

    #[test]
    fn rewrites_single_entry_images_to_backend_relative_download_urls() {
        let entry = DiaryEntryResponseDto {
            id: String::from("entry-1"),
            date: String::from("2026-05-01T00:00:00Z"),
            title: String::from("Title"),
            content: String::from("Content"),
            tags: vec![String::from("Harvest")],
            images: vec![sample_image()],
            created_at: String::from("2026-05-01T00:00:00Z"),
            updated_at: String::from("2026-05-01T00:00:00Z"),
        };

        let rewritten = with_backend_relative_download_urls(entry);

        assert_eq!(rewritten.images[0].download_url, "/api/diary/entry-1/images/img-1");
    }

    #[test]
    fn rewrites_list_entry_images_to_backend_relative_download_urls() {
        let response = GetDiaryResponseDto {
            entries: vec![DiaryEntryResponseDto {
                id: String::from("entry-1"),
                date: String::from("2026-05-01T00:00:00Z"),
                title: String::from("Title"),
                content: String::from("Content"),
                tags: vec![],
                images: vec![sample_image()],
                created_at: String::from("2026-05-01T00:00:00Z"),
                updated_at: String::from("2026-05-01T00:00:00Z"),
            }],
        };

        let rewritten = with_backend_relative_download_urls_for_list(response);

        assert_eq!(rewritten.entries[0].images[0].download_url, "/api/diary/entry-1/images/img-1");
    }

    #[tokio::test]
    async fn forwards_tag_aware_diary_queries_and_rewrites_download_urls() {
        let server = spawn_test_server().await;
        let query = GetDiaryEntriesQueryDto {
            start: String::from("2026-05-01T00:00:00Z"),
            end: String::from("2026-05-02T00:00:00Z"),
            tags: vec![String::from("Tomatoes"), String::from("Harvest")],
            tag_filter_mode: DiaryTagFilterModeDto::All,
        };

        let response = get_diary(&server.base_url, &query).await.unwrap();

        let captured_query = server
            .state
            .captured_query
            .lock()
            .unwrap()
            .clone()
            .unwrap();

        assert_eq!(captured_query, query);
        assert_eq!(response.entries[0].images[0].download_url, "/api/diary/entry-1/images/img-1");
    }

    #[tokio::test]
    async fn forwards_image_upload_using_shared_route_contract() {
        let server = spawn_test_server().await;
        let entry_id = uuid!("11111111-1111-1111-1111-111111111111");
        let mut headers = HeaderMap::new();
        headers.insert(FILE_NAME_HEADER, "leaf.png".parse().unwrap());
        headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());

        let metadata = upload_diary_image(&server.base_url, entry_id, &headers, Bytes::from_static(&[1, 2, 3]))
            .await
            .unwrap();

        let captured_upload = server
            .state
            .captured_upload
            .lock()
            .unwrap()
            .clone()
            .unwrap();

        assert_eq!(
            captured_upload,
            CapturedUpload {
                entry_id: String::from("11111111-1111-1111-1111-111111111111"),
                file_name: String::from("leaf.png"),
                media_type: String::from("image/png"),
                body: vec![1, 2, 3],
            }
        );
        assert_eq!(metadata.download_url, "/api/diary/11111111-1111-1111-1111-111111111111/images/img-1");
    }

    #[tokio::test]
    async fn forwards_image_download_using_shared_route_contract() {
        let server = spawn_test_server().await;
        let entry_id = uuid!("11111111-1111-1111-1111-111111111111");
        let image_id = uuid!("22222222-2222-2222-2222-222222222222");

        let response = download_diary_image(&server.base_url, entry_id, image_id)
            .await
            .unwrap();

        let captured_download = server
            .state
            .captured_download
            .lock()
            .unwrap()
            .clone()
            .unwrap();

        assert_eq!(
            captured_download,
            (
                String::from("11111111-1111-1111-1111-111111111111"),
                String::from("22222222-2222-2222-2222-222222222222"),
            )
        );
        assert_eq!(response.media_type, "image/jpeg");
        assert_eq!(response.bytes, vec![9_u8, 8, 7]);
    }

    #[tokio::test]
    async fn forwards_image_delete_using_shared_route_contract() {
        let server = spawn_test_server().await;
        let entry_id = uuid!("11111111-1111-1111-1111-111111111111");
        let image_id = uuid!("22222222-2222-2222-2222-222222222222");

        delete_diary_image(&server.base_url, entry_id, image_id)
            .await
            .unwrap();

        let captured_delete = server
            .state
            .captured_delete
            .lock()
            .unwrap()
            .clone()
            .unwrap();

        assert_eq!(
            captured_delete,
            (
                String::from("11111111-1111-1111-1111-111111111111"),
                String::from("22222222-2222-2222-2222-222222222222"),
            )
        );
    }
}
