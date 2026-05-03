use greenhouse_core::data_storage_service_dto::diary_dtos::get_diary::GetDiaryResponseDto;
use greenhouse_core::data_storage_service_dto::diary_dtos::post_diary_entry::PostDiaryEntryDtoRequest;
use greenhouse_core::data_storage_service_dto::diary_dtos::put_diary_entry::PutDiaryEntryDtoRequest;
use test_helper::TestContext;
mod test_helper;

#[tokio::test]
async fn test_create_and_get_diary_entry() {
    let mut context = TestContext::new();
    context.start_all_services().await;
    let token = test_helper::admin_login().await;

    // Create diary entry
    let client = reqwest::Client::new();
    let post_entry = PostDiaryEntryDtoRequest {
        title: "Test Entry".to_string(),
        content: "This is a test diary entry.".to_string(),
        date: chrono::Utc::now().to_string(),
        tags: Vec::new(),
    };
    let response = client
        .post("http://localhost:3000/api/diary")
        .json(&post_entry)
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to create diary entry"
    );

    // Get diary entries for a date range
    let response = client
        .get(format!(
            "http://localhost:3000/api/diary/{}/{}",
            post_entry.date, post_entry.date
        ))
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to get diary entries"
    );
    let entry = response.json::<GetDiaryResponseDto>().await.unwrap();
    assert!(!entry.entries.is_empty(), "No diary entries returned");

    // Get a specific diary entry by id
    let response = client
        .get(format!(
            "http://localhost:3000/api/diary/{}",
            entry.entries[0].id
        ))
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to get diary entry by id"
    );

    context.stop().await;
}

#[tokio::test]
async fn test_update_diary_entry() {
    let mut context = TestContext::new();
    context.start_all_services().await;
    let token = test_helper::admin_login().await;

    // Create diary entry
    let client = reqwest::Client::new();
    let post_entry = PostDiaryEntryDtoRequest {
        title: "Entry to Update".to_string(),
        content: "Original content.".to_string(),
        date: chrono::Utc::now().to_string(),
        tags: Vec::new(),
    };
    let response = client
        .post("http://localhost:3000/api/diary")
        .json(&post_entry)
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to create diary entry"
    );

    // Get diary entries for a date range
    let response = client
        .get(format!(
            "http://localhost:3000/api/diary/{}/{}",
            post_entry.date, post_entry.date
        ))
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    let entry = response.json::<GetDiaryResponseDto>().await.unwrap();

    // Update the diary entry
    let update_entry = PutDiaryEntryDtoRequest {
        title: String::from("Updated Title"),
        content: String::from("Updated content."),
        date: chrono::Utc::now().to_string(),
        tags: Vec::new(),
    };
    let response = client
        .put(format!(
            "http://localhost:3000/api/diary/{}",
            entry.entries[0].id
        ))
        .json(&update_entry)
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "Failed to update diary entry"
    );

    // Get the updated diary entry
    let response = client
        .get(format!(
            "http://localhost:3000/api/diary/{}",
            entry.entries[0].id
        ))
        .header("Access-Control-Allow-Credentials", "true")
        .header("Cookie", format!("auth-token={token}"))
        .send()
        .await
        .unwrap();
    let entry: serde_json::Value = response.json().await.unwrap();
    assert_eq!(entry["title"], "Updated Title");
    assert_eq!(entry["content"], "Updated content.");

    context.stop().await;
}
