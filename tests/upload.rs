use reqwest::{Client, multipart};
use serde_json::json;
use tokio::test;
use uuid::Uuid;

/// Helper function to get the base URL of the API
fn api_base_url() -> String {
    "http://127.0.0.1:3000/api".to_string()
}

#[tokio::test]
async fn test_upload_image_success() {
    let client = Client::new();
    let file_name = "test_image.jpg";
    let file_content = "fake_image_data";

    let form = multipart::Form::new()
        .text("file_name", file_name.to_string())
        .part(
            "file",
            multipart::Part::bytes(file_content.as_bytes().to_vec())
                .file_name(file_name)
                .mime_str("image/jpeg")
                .unwrap(),
        );

    let response = client
        .post(&format!("{}/image", api_base_url()))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["message"], "File uploaded successfully.");
}

#[tokio::test]
async fn test_upload_image_unsupported_type() {
    let client = Client::new();
    let file_name = "test_image.txt";
    let file_content = "fake_image_data";

    let form = multipart::Form::new()
        .text("file_name", file_name.to_string())
        .part(
            "file",
            multipart::Part::bytes(file_content.as_bytes().to_vec())
                .file_name(file_name)
                .mime_str("text/plain")
                .unwrap(),
        );

    let response = client
        .post(&format!("{}/image", api_base_url()))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "Unsupported content type.");
}

#[tokio::test]
async fn test_get_images() {
    let client = Client::new();
    let response = client
        .get(&format!("{}/image", api_base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_get_image_by_id() {
    let client = Client::new();
    let image_id = 1;

    let response = client
        .get(&format!("{}/image/{}", api_base_url(), image_id))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status() == 400);
}

#[tokio::test]
async fn test_patch_image() {
    let client = Client::new();
    let image_id = 1;

    let payload = json!({
        "file_name": "updated_image_name.jpg"
    });

    let response = client
        .patch(&format!("{}/admin/image/{}", api_base_url(), image_id))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status() == 400);
}

#[tokio::test]
async fn test_delete_image() {
    let client = Client::new();
    let image_id = 1;

    let response = client
        .delete(&format!("{}/admin/image/{}", api_base_url(), image_id))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status() == 400);
}
