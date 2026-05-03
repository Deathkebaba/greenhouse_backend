use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::{
    GetDiaryEntriesQueryDto, GetDiaryResponseDto,
};
use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary_entry::DiaryEntryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::post_diary_entry::PostDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::put_diary_entry::PutDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::query::DiaryTagFilterModeDto;
use once_cell::sync::Lazy;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use tokio::sync::Mutex;
use test_helper::TestContext;

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
    assert_eq!(sorted_strings(created_entry.tags.clone()), sorted_strings(vec![String::from("Tomatoes"), String::from("Harvest")]));
    assert!(created_entry.images.is_empty(), "expected no images on a new entry");

    let image_bytes = vec![0_u8, 1, 2, 3, 4, 5, 6, 7];
    let upload_response = authenticated(
        client.post(format!("{API_BASE_URL}/{}/images", created_entry.id)),
        &token,
    )
    .header(FILE_NAME_HEADER, "sprouts.png")
    .header(CONTENT_TYPE, "image/png")
    .body(image_bytes.clone())
    .send()
    .await
    .unwrap();
    assert!(
        upload_response.status().is_success(),
        "Failed to upload diary image"
    );

    let uploaded_image: serde_json::Value = upload_response.json().await.unwrap();
    assert_eq!(uploaded_image["file_name"], "sprouts.png");
    assert_eq!(uploaded_image["media_type"], "image/png");
    assert_eq!(uploaded_image["byte_size"], image_bytes.len() as i64);

    let image_id = uploaded_image["id"].as_str().unwrap().to_string();
    let download_url = uploaded_image["download_url"].as_str().unwrap().to_string();
    assert_eq!(
        download_url,
        format!("/api/diary/{}/images/{}", created_entry.id, image_id)
    );

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
    assert_eq!(updated_entry.images.len(), 1);
    assert_eq!(updated_entry.images[0].id, image_id);
    assert_eq!(updated_entry.images[0].file_name, "sprouts.png");
    assert_eq!(updated_entry.images[0].media_type, "image/png");
    assert_eq!(updated_entry.images[0].byte_size, image_bytes.len() as i64);
    assert_eq!(updated_entry.images[0].download_url, download_url);

    let download_response = authenticated(
        client.get(format!("http://localhost:3000{download_url}")),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert!(
        download_response.status().is_success(),
        "Failed to download diary image"
    );
    assert_eq!(
        download_response
            .headers()
            .get(CONTENT_TYPE)
            .unwrap(),
        "image/png"
    );
    assert_eq!(download_response.bytes().await.unwrap().as_ref(), image_bytes.as_slice());

    let delete_response = authenticated(
        client.delete(format!("{API_BASE_URL}/{}/images/{}", created_entry.id, image_id)),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(delete_response.status(), reqwest::StatusCode::NO_CONTENT);

    let entry_after_delete = get_entry(&client, &token, &created_entry.id).await;
    assert!(
        entry_after_delete.images.is_empty(),
        "expected image metadata to be removed after delete"
    );

    let download_after_delete = authenticated(
        client.get(format!("http://localhost:3000{download_url}")),
        &token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(download_after_delete.status(), reqwest::StatusCode::NOT_FOUND);

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
    assert_eq!(any_match.entries.len(), 3, "any-tag query should match all relevant entries");

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
    assert_eq!(all_match.entries.len(), 1, "all-tags query should only match entries containing both tags");
    assert_eq!(all_match.entries[0].title, "Tomatoes and harvest");
    assert_eq!(
        sorted_strings(all_match.entries[0].tags.clone()),
        sorted_strings(vec![String::from("Tomatoes"), String::from("HARVEST")])
    );

    context.stop().await;
}

fn authenticated(
    request: reqwest::RequestBuilder,
    token: &str,
) -> reqwest::RequestBuilder {
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

    assert!(response.status().is_success(), "Failed to create diary entry");
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

    assert!(response.status().is_success(), "Failed to query diary entries");
    response.json::<GetDiaryResponseDto>().await.unwrap()
}

async fn get_entry(client: &reqwest::Client, token: &str, entry_id: &str) -> DiaryEntryResponseDto {
    let response = authenticated(client.get(format!("{API_BASE_URL}/{entry_id}")), token)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Failed to get diary entry by id");
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
