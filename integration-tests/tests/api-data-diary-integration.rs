use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::{
    GetDiaryEntriesQueryDto, GetDiaryResponseDto,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary_entry::DiaryEntryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::image_metadata::DiaryImageMetadataDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::post_diary_entry::PostDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::put_diary_entry::PutDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::query::DiaryTagFilterModeDto;
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use test_helper::TestContext;
use tokio::sync::Mutex;

mod test_helper;

const API_BASE_URL: &str = "http://localhost:3000/api/diary";
const FILE_NAME_HEADER: &str = "x-file-name";
static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[tokio::test]
async fn test_create_update_and_manage_diary_images_with_tags() {
    let _guard = TEST_MUTEX.lock().await;
    let mut context = TestContext::new();
    context.start_all_services().await;
    let token = test_helper::admin_login().await;
    let client = reqwest::Client::new();

    let post_entry = PostDiaryEntryDtoRequest {
        title: String::from("Tomato diary"),
        content: String::from("Seedlings transplanted into the raised bed."),
        date: String::from("2026-05-01T10:30:00Z"),
        tags: vec![String::from("Tomatoes"), String::from(" Harvest ")],
    };

    let response = authenticated(client.post(API_BASE_URL), &token)
        .json(&post_entry)
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to create diary entry"
    );

    let created_entry = get_single_entry(
        &client,
        &token,
        GetDiaryEntriesQueryDto {
            start: String::from("2026-05-01T00:00:00Z"),
            end: String::from("2026-05-02T00:00:00Z"),
            tags: vec![String::from("tomatoes")],
            tag_filter_mode: DiaryTagFilterModeDto::Any,
        },
        |entry| entry.title == post_entry.title,
    )
    .await;

    assert_eq!(created_entry.content, post_entry.content);
    assert_eq!(
        sorted_strings(created_entry.tags.clone()),
        sorted_strings(vec![String::from("Tomatoes"), String::from("Harvest")])
    );
    assert!(
        created_entry.images.is_empty(),
        "expected no images on a new entry"
    );

    let first_image_bytes = vec![0_u8, 1, 2, 3, 4, 5, 6, 7];
    let first_image = upload_image(
        &client,
        &token,
        &created_entry.id,
        "sprouts.png",
        "image/png",
        first_image_bytes.clone(),
    )
    .await;
    let second_image_bytes = vec![255_u8, 216, 255, 224, 0, 16, 74, 70, 73, 70];
    let second_image = upload_image(
        &client,
        &token,
        &created_entry.id,
        "garden.jpg",
        "image/jpeg",
        second_image_bytes.clone(),
    )
    .await;

    let updated_entry_request = PutDiaryEntryDtoRequest {
        title: String::from("Tomato diary updated"),
        content: String::from("Seedlings watered and tagged for pruning."),
        date: String::from("2026-05-01T12:00:00Z"),
        tags: vec![String::from("Tomatoes"), String::from("Pruning")],
    };
    let update_response = authenticated(
        client.put(format!("{API_BASE_URL}/{}", created_entry.id)),
        &token,
    )
    .json(&updated_entry_request)
    .send()
    .await
    .unwrap();
    assert!(
        update_response.status().is_success(),
        "Failed to update diary entry"
    );

    let updated_entry = get_entry(&client, &token, &created_entry.id).await;
    assert_eq!(updated_entry.title, updated_entry_request.title);
    assert_eq!(updated_entry.content, updated_entry_request.content);
    assert_eq!(
        sorted_strings(updated_entry.tags.clone()),
        sorted_strings(vec![String::from("Tomatoes"), String::from("Pruning")])
    );
    assert_eq!(updated_entry.images.len(), 2);
    assert_same_images(
        &updated_entry.images,
        &[first_image.clone(), second_image.clone()],
    );

    assert_download_matches(
        &client,
        &token,
        &first_image.download_url,
        "image/png",
        &first_image_bytes,
    )
    .await;
    assert_download_matches(
        &client,
        &token,
        &second_image.download_url,
        "image/jpeg",
        &second_image_bytes,
    )
    .await;

    let delete_response = authenticated(
        client.delete(format!(
            "{API_BASE_URL}/{}/images/{}",
            created_entry.id, first_image.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(delete_response.status(), reqwest::StatusCode::NO_CONTENT);

    let entry_after_delete = get_entry(&client, &token, &created_entry.id).await;
    assert_eq!(entry_after_delete.images.len(), 1);
    assert_same_images(&entry_after_delete.images, &[second_image.clone()]);

    let download_after_delete = authenticated(
        client.get(format!("http://localhost:3000{}", first_image.download_url)),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(download_after_delete.status(), StatusCode::NOT_FOUND);

    assert_download_matches(
        &client,
        &token,
        &second_image.download_url,
        "image/jpeg",
        &second_image_bytes,
    )
    .await;

    context.stop().await;
}

#[tokio::test]
async fn test_diary_image_error_paths() {
    let _guard = TEST_MUTEX.lock().await;
    let mut context = TestContext::new();
    context.start_all_services().await;
    let token = test_helper::admin_login().await;
    let client = reqwest::Client::new();

    let created_entry = create_entry_and_get(
        &client,
        &token,
        PostDiaryEntryDtoRequest {
            title: String::from("Image failures"),
            content: String::from("Entry used for diary image error-path coverage."),
            date: String::from("2026-05-03T08:00:00Z"),
            tags: vec![String::from("Failures")],
        },
    )
    .await;
    let uploaded_image = upload_image(
        &client,
        &token,
        &created_entry.id,
        "existing.png",
        "image/png",
        vec![1_u8, 3, 3, 7],
    )
    .await;

    let missing_entry_id = "11111111-1111-1111-1111-111111111111";
    let missing_image_id = "22222222-2222-2222-2222-222222222222";

    let missing_entry_upload = authenticated(
        client.post(format!("{API_BASE_URL}/{missing_entry_id}/images")),
        &token,
    )
    .header(FILE_NAME_HEADER, "missing.png")
    .header(CONTENT_TYPE, "image/png")
    .body(vec![9_u8, 9, 9])
    .send()
    .await
    .unwrap();
    assert_eq!(missing_entry_upload.status(), StatusCode::NOT_FOUND);

    let missing_entry_download = authenticated(
        client.get(format!(
            "{API_BASE_URL}/{missing_entry_id}/images/{}",
            uploaded_image.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(missing_entry_download.status(), StatusCode::NOT_FOUND);

    let missing_entry_delete = authenticated(
        client.delete(format!(
            "{API_BASE_URL}/{missing_entry_id}/images/{}",
            uploaded_image.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(missing_entry_delete.status(), StatusCode::NOT_FOUND);

    let missing_image_download = authenticated(
        client.get(format!(
            "{API_BASE_URL}/{}/images/{missing_image_id}",
            created_entry.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(missing_image_download.status(), StatusCode::NOT_FOUND);

    let missing_image_delete = authenticated(
        client.delete(format!(
            "{API_BASE_URL}/{}/images/{missing_image_id}",
            created_entry.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(missing_image_delete.status(), StatusCode::NOT_FOUND);

    let missing_file_name = authenticated(
        client.post(format!("{API_BASE_URL}/{}/images", created_entry.id)),
        &token,
    )
    .header(CONTENT_TYPE, "image/png")
    .body(vec![5_u8, 4, 3, 2])
    .send()
    .await
    .unwrap();
    assert_eq!(missing_file_name.status(), StatusCode::BAD_REQUEST);

    let missing_content_type = authenticated(
        client.post(format!("{API_BASE_URL}/{}/images", created_entry.id)),
        &token,
    )
    .header(FILE_NAME_HEADER, "missing-content-type.png")
    .body(vec![5_u8, 4, 3, 2])
    .send()
    .await
    .unwrap();
    assert_eq!(missing_content_type.status(), StatusCode::BAD_REQUEST);

    let empty_body = authenticated(
        client.post(format!("{API_BASE_URL}/{}/images", created_entry.id)),
        &token,
    )
    .header(FILE_NAME_HEADER, "empty.png")
    .header(CONTENT_TYPE, "image/png")
    .body(Vec::new())
    .send()
    .await
    .unwrap();
    assert_eq!(empty_body.status(), StatusCode::BAD_REQUEST);

    let blank_file_name = authenticated(
        client.post(format!("{API_BASE_URL}/{}/images", created_entry.id)),
        &token,
    )
    .header(FILE_NAME_HEADER, "   ")
    .header(CONTENT_TYPE, "image/png")
    .body(vec![8_u8, 6, 7, 5, 3, 0, 9])
    .send()
    .await
    .unwrap();
    assert_eq!(blank_file_name.status(), StatusCode::BAD_REQUEST);

    let delete_existing = authenticated(
        client.delete(format!(
            "{API_BASE_URL}/{}/images/{}",
            created_entry.id, uploaded_image.id
        )),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(delete_existing.status(), StatusCode::NO_CONTENT);

    context.stop().await;
}

#[tokio::test]
async fn test_filters_diary_entries_by_any_and_all_tags() {
    let _guard = TEST_MUTEX.lock().await;
    let mut context = TestContext::new();
    context.start_all_services().await;
    let token = test_helper::admin_login().await;
    let client = reqwest::Client::new();

    create_entry(
        &client,
        &token,
        PostDiaryEntryDtoRequest {
            title: String::from("Tomatoes only"),
            content: String::from("Only tomato tag present."),
            date: String::from("2026-05-10T08:00:00Z"),
            tags: vec![String::from("Tomatoes")],
        },
    )
    .await;
    create_entry(
        &client,
        &token,
        PostDiaryEntryDtoRequest {
            title: String::from("Harvest and basil"),
            content: String::from("Basket weighed after picking."),
            date: String::from("2026-05-11T09:00:00Z"),
            tags: vec![String::from("Harvest"), String::from("Basil")],
        },
    )
    .await;
    create_entry(
        &client,
        &token,
        PostDiaryEntryDtoRequest {
            title: String::from("Tomatoes and harvest"),
            content: String::from("Ripe tomatoes harvested before rain."),
            date: String::from("2026-05-12T18:45:00Z"),
            tags: vec![String::from(" Tomatoes "), String::from("HARVEST")],
        },
    )
    .await;

    let any_match = get_diary(
        &client,
        &token,
        &GetDiaryEntriesQueryDto {
            start: String::from("2026-05-10T00:00:00Z"),
            end: String::from("2026-05-13T00:00:00Z"),
            tags: vec![String::from("tomatoes"), String::from("harvest")],
            tag_filter_mode: DiaryTagFilterModeDto::Any,
        },
    )
    .await;
    assert_eq!(
        any_match.entries.len(),
        3,
        "any-tag query should match all relevant entries"
    );

    let all_match = get_diary(
        &client,
        &token,
        &GetDiaryEntriesQueryDto {
            start: String::from("2026-05-10T00:00:00Z"),
            end: String::from("2026-05-13T00:00:00Z"),
            tags: vec![String::from(" harvest "), String::from("TOMATOES")],
            tag_filter_mode: DiaryTagFilterModeDto::All,
        },
    )
    .await;
    assert_eq!(
        all_match.entries.len(),
        1,
        "all-tags query should only match entries containing both tags"
    );
    assert_eq!(all_match.entries[0].title, "Tomatoes and harvest");
    assert_eq!(
        sorted_strings(all_match.entries[0].tags.clone()),
        sorted_strings(vec![String::from("Tomatoes"), String::from("HARVEST")])
    );

    context.stop().await;
}

fn authenticated(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .header("Access-Control-Allow-Credentials", "true")
        .header(COOKIE, format!("auth-token={token}"))
}

fn sorted_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

async fn create_entry(client: &reqwest::Client, token: &str, entry: PostDiaryEntryDtoRequest) {
    let response = authenticated(client.post(API_BASE_URL), token)
        .json(&entry)
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Failed to create diary entry"
    );
}

async fn create_entry_and_get(
    client: &reqwest::Client,
    token: &str,
    entry: PostDiaryEntryDtoRequest,
) -> DiaryEntryResponseDto {
    let title = entry.title.clone();
    let date = entry.date.clone();
    create_entry(client, token, entry).await;
    get_single_entry(
        client,
        token,
        GetDiaryEntriesQueryDto {
            start: date.clone(),
            end: increment_iso8601_second(&date),
            tags: Vec::new(),
            tag_filter_mode: DiaryTagFilterModeDto::Any,
        },
        |candidate| candidate.title == title,
    )
    .await
}

async fn upload_image(
    client: &reqwest::Client,
    token: &str,
    entry_id: &str,
    file_name: &str,
    media_type: &str,
    body: Vec<u8>,
) -> DiaryImageMetadataDto {
    let response = authenticated(
        client.post(format!("{API_BASE_URL}/{entry_id}/images")),
        token,
    )
    .header(FILE_NAME_HEADER, file_name)
    .header(CONTENT_TYPE, media_type)
    .body(body.clone())
    .send()
    .await
    .unwrap();

    assert!(
        response.status().is_success(),
        "Failed to upload diary image"
    );

    let metadata = response.json::<DiaryImageMetadataDto>().await.unwrap();
    assert_eq!(metadata.file_name, file_name);
    assert_eq!(metadata.media_type, media_type);
    assert_eq!(metadata.byte_size, body.len() as i64);
    assert_eq!(
        metadata.download_url,
        format!("/api/diary/{entry_id}/images/{}", metadata.id)
    );

    metadata
}

async fn assert_download_matches(
    client: &reqwest::Client,
    token: &str,
    download_url: &str,
    expected_media_type: &str,
    expected_body: &[u8],
) {
    let response = authenticated(
        client.get(format!("http://localhost:3000{download_url}")),
        token,
    )
    .send()
    .await
    .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to download diary image"
    );
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        expected_media_type
    );
    assert_eq!(response.bytes().await.unwrap().as_ref(), expected_body);
}

fn assert_same_images(actual: &[DiaryImageMetadataDto], expected: &[DiaryImageMetadataDto]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "unexpected number of diary images"
    );

    let mut actual_images = actual.to_vec();
    let mut expected_images = expected.to_vec();
    actual_images.sort_by(|left, right| left.id.cmp(&right.id));
    expected_images.sort_by(|left, right| left.id.cmp(&right.id));

    for (actual_image, expected_image) in actual_images.iter().zip(expected_images.iter()) {
        assert_eq!(actual_image.id, expected_image.id);
        assert_eq!(actual_image.file_name, expected_image.file_name);
        assert_eq!(actual_image.media_type, expected_image.media_type);
        assert_eq!(actual_image.byte_size, expected_image.byte_size);
        assert_eq!(actual_image.download_url, expected_image.download_url);
        assert!(
            !actual_image.uploaded_at.is_empty(),
            "uploaded_at should be present for diary images"
        );
    }
}

fn increment_iso8601_second(timestamp: &str) -> String {
    let date_time = chrono::DateTime::parse_from_rfc3339(timestamp).unwrap();
    (date_time + chrono::Duration::seconds(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

async fn get_diary(
    client: &reqwest::Client,
    token: &str,
    query: &GetDiaryEntriesQueryDto,
) -> GetDiaryResponseDto {
    let response = authenticated(client.get(API_BASE_URL), token)
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

async fn get_entry(client: &reqwest::Client, token: &str, entry_id: &str) -> DiaryEntryResponseDto {
    let response = authenticated(client.get(format!("{API_BASE_URL}/{entry_id}")), token)
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Failed to get diary entry by id"
    );
    response.json::<DiaryEntryResponseDto>().await.unwrap()
}

async fn get_single_entry<F>(
    client: &reqwest::Client,
    token: &str,
    query: GetDiaryEntriesQueryDto,
    predicate: F,
) -> DiaryEntryResponseDto
where
    F: Fn(&DiaryEntryResponseDto) -> bool,
{
    let response = get_diary(client, token, &query).await;
    response
        .entries
        .into_iter()
        .find(predicate)
        .expect("Expected to find matching diary entry")
}
