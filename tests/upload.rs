use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{multipart, Client, StatusCode};
use serde_json::{json, Value};
use tokio;

#[tokio::test]
async fn test_upload_image() {
    let client = Client::new();

    // Step 1: Login as Admin
    let login_payload = json!({
        "username": "admin",
        "password": "12345"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), reqwest::StatusCode::OK);

    let body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Prepare multipart form data
    let form = multipart::Form::new()
        .file("file", "/workspaces/rust-baranki/uploads/DMAAAgIvNOA-1920.jpg")
        .await
        .expect("Failed to attach file");

    // Step 3: Upload image
    let upload_response = client
        .post("http://127.0.0.1:3000/api/image")
        .bearer_auth(token) // Use Authorization header with Bearer token
        .multipart(form)
        .send()
        .await
        .expect("Failed to send upload request");

    // Step 4: Validate response
    assert_eq!(upload_response.status(), reqwest::StatusCode::CREATED);

    let response_body = upload_response.text().await.expect("Failed to read response body");
    println!("Response body: {}", response_body);
}

#[tokio::test]
async fn test_get_image_by_id() {
    let client = Client::new();

    // Step 1: Login as Admin
    let login_payload = json!({
        "username": "admin",
        "password": "12345"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let body = login_response
        .json::<Value>()
        .await
        .expect("Failed to parse login response JSON");
    let token = body["token"]
        .as_str()
        .expect("Token not found in login response");

    println!("token: {:?}", token);
    // Step 2: Get Image by ID
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to set AUTHORIZATION header"),
    );

    let image_id = 1; // Replace with valid image ID
    let get_response = client
        .get(format!("http://127.0.0.1:3000/image/{}", image_id))
        .headers(headers.clone())
        .send()
        .await
        .expect("Failed to send GET image request");

    assert_eq!(get_response.status(), StatusCode::OK);
    /*
    let get_body = get_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse GET image response JSON");

    println!("Get Image Response: {:?}", get_body);
    */
    //expecting file to be really file. "trust me bro"
}

#[tokio::test]
async fn test_patch_image() {
    let client = Client::new();

    // Step 1: Login as Admin
    let login_payload = json!({
        "username": "admin",
        "password": "12345"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");
    let token = body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Patch Image
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to set AUTHORIZATION header"),
    );

    let patch_payload = json!({
        "file_name": "updated_image_name"
    });

    let image_id = 1; // Replace with valid image ID
    let patch_response = client
        .patch(format!("http://127.0.0.1:3000/api/image/{}", image_id))
        .headers(headers.clone())
        .json(&patch_payload)
        .send()
        .await
        .expect("Failed to send PATCH image request");

    assert_eq!(patch_response.status(), StatusCode::OK);
    let patch_body = patch_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse PATCH image response JSON");

    println!("Patch Image Response: {:?}", patch_body);
}

#[tokio::test]
async fn test_delete_image() {
    let client = Client::new();

    // Step 1: Login as Admin
    let login_payload = json!({
        "username": "admin",
        "password": "12345"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");
    let token = body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Delete Image
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to set AUTHORIZATION header"),
    );

    let image_id = 1; // Replace with valid image ID
    let delete_response = client
        .delete(format!("http://127.0.0.1:3000/api/image/{}", image_id))
        .headers(headers)
        .send()
        .await
        .expect("Failed to send DELETE image request");

    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_body = delete_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse DELETE image response JSON");

    println!("Delete Image Response: {:?}", delete_body);
}
