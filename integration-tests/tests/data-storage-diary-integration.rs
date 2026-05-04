use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::{
    GetDiaryEntriesQueryDto, GetDiaryResponseDto,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary_entry::DiaryEntryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::image_metadata::DiaryImageMetadataDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::query::DiaryTagFilterModeDto;
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use reqwest::header::CONTENT_TYPE;
use test_helper::TestContext;
use tokio::sync::Mutex;
use uuid::Uuid;

mod test_helper;

const DATA_STORAGE_BASE_URL: &str = "http://localhost:3002/diary";
const FILE_NAME_HEADER: &str = "x-file-name";
static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[tokio::test]
async fn persists_tags_filters_entries_and_round_trips_images() {
    let _guard = TEST_MUTEX.lock().await;
    let mut context = TestContext::new();
    context.start_all_services().await;
    let client = reqwest::Client::new();

    let created = client
        .post(format!("{DATA_STORAGE_BASE_URL}/"))
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

    assert_eq!(
        created.tags,
        vec![String::from("Harvest"), String::from("Tomatoes")]
    );
    assert!(created.images.is_empty());

    client
        .post(format!("{DATA_STORAGE_BASE_URL}/"))
        .json(&serde_json::json!({
            "date": "2026-05-03T11:00:00Z",
            "title": "Cucumbers",
            "content": "Checked growth",
            "tags": ["Inspection"]
        }))
        .send()
        .await
        .unwrap();

    let any_filter = get_diary(
        &client,
        &GetDiaryEntriesQueryDto {
            start: String::from("2026-05-03T00:00:00Z"),
            end: String::from("2026-05-04T00:00:00Z"),
            tags: vec![String::from("tomatoes")],
            tag_filter_mode: DiaryTagFilterModeDto::Any,
        },
    )
    .await;
    assert_eq!(any_filter.entries.len(), 1);
    assert_eq!(any_filter.entries[0].id, created.id);

    let all_filter = get_diary(
        &client,
        &GetDiaryEntriesQueryDto {
            start: String::from("2026-05-03T00:00:00Z"),
            end: String::from("2026-05-04T00:00:00Z"),
            tags: vec![String::from("harvest"), String::from("tomatoes")],
            tag_filter_mode: DiaryTagFilterModeDto::All,
        },
    )
    .await;
    assert_eq!(all_filter.entries.len(), 1);
    assert_eq!(all_filter.entries[0].id, created.id);

    let image_bytes = vec![137, 80, 78, 71];
    let uploaded = client
        .post(format!("{DATA_STORAGE_BASE_URL}/{}/images", created.id))
        .header(FILE_NAME_HEADER, "leaf.png")
        .header(CONTENT_TYPE, "image/png")
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
        .get(format!("{DATA_STORAGE_BASE_URL}/{}", created.id))
        .send()
        .await
        .unwrap()
        .json::<DiaryEntryResponseDto>()
        .await
        .unwrap();
    assert_eq!(fetched_entry.images, vec![uploaded_metadata.clone()]);

    let downloaded = client
        .get(format!(
            "{DATA_STORAGE_BASE_URL}/{}/images/{}",
            created.id, uploaded_metadata.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(downloaded.status(), StatusCode::OK);
    assert_eq!(
        downloaded.headers()[CONTENT_TYPE],
        reqwest::header::HeaderValue::from_static("image/png")
    );
    assert_eq!(downloaded.bytes().await.unwrap().to_vec(), image_bytes);

    let deleted = client
        .delete(format!(
            "{DATA_STORAGE_BASE_URL}/{}/images/{}",
            created.id, uploaded_metadata.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let after_delete = client
        .get(format!("{DATA_STORAGE_BASE_URL}/{}", created.id))
        .send()
        .await
        .unwrap()
        .json::<DiaryEntryResponseDto>()
        .await
        .unwrap();
    assert!(after_delete.images.is_empty());

    context.stop().await;
}

#[tokio::test]
async fn image_routes_return_not_found_for_missing_resources() {
    let _guard = TEST_MUTEX.lock().await;
    let mut context = TestContext::new();
    context.start_all_services().await;
    let client = reqwest::Client::new();
    let entry_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();

    let missing_upload = client
        .post(format!("{DATA_STORAGE_BASE_URL}/{entry_id}/images"))
        .header(FILE_NAME_HEADER, "missing.png")
        .header(CONTENT_TYPE, "image/png")
        .body(vec![1, 2, 3])
        .send()
        .await
        .unwrap();
    assert_eq!(missing_upload.status(), StatusCode::NOT_FOUND);

    let missing_download = client
        .get(format!(
            "{DATA_STORAGE_BASE_URL}/{entry_id}/images/{image_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_download.status(), StatusCode::NOT_FOUND);

    let missing_delete = client
        .delete(format!(
            "{DATA_STORAGE_BASE_URL}/{entry_id}/images/{image_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_delete.status(), StatusCode::NOT_FOUND);

    context.stop().await;
}

async fn get_diary(
    client: &reqwest::Client,
    query: &GetDiaryEntriesQueryDto,
) -> GetDiaryResponseDto {
    let response = client
        .get(format!("{DATA_STORAGE_BASE_URL}/"))
        .query(query)
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Failed to query diary entries"
    );
    response.json::<GetDiaryResponseDto>().await.unwrap()
}
