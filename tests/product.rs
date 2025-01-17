use reqwest::{header, Client, StatusCode};
use serde_json::json;
use tokio;

#[tokio::test]
async fn test_create_product() {
    let client = Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "admin",
        "password": "Muzion15"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = login_body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Set Authorization Header
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to create Authorization header"),
    );

    // Step 3: Define payload for creating a product
    let create_payload = json!({
        "name": "Test Product",
        "price": 100.0,
        "description": "A test product",
        "image_id": 1,
        "category_id": 1,
        "is_featured": true,
        "is_available": true
    });

    // Step 4: Send request to create product
    let create_response = client
        .post("http://127.0.0.1:3000/api/admin/product")
        .headers(headers.clone())
        .json(&create_payload)
        .send()
        .await
        .expect("Failed to send create product request");

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body = create_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse create product response JSON");

    assert_eq!(
        create_body["message"].as_str(),
        Some("Product created successfully")
    );
}

#[tokio::test]
async fn test_get_products() {
    let client = Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "admin",
        "password": "Muzion15"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = login_body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Set Authorization Header
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to create Authorization header"),
    );

    // Step 3: Send request to get products
    let response = client
        .get("http://127.0.0.1:3000/api/product")
        .headers(headers)
        .send()
        .await
        .expect("Failed to send get products request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse get products response JSON");

    // Ensure response is a JSON array (even if empty)
    assert!(body.is_array());
}

#[tokio::test]
async fn test_get_product() {
    let client = Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "admin",
        "password": "Muzion15"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = login_body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Set Authorization Header
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to create Authorization header"),
    );

    // Step 3: Get product by id (replace 1 with an actual product id)
    let response = client
        .get("http://127.0.0.1:3000/api/product/1")
        .headers(headers)
        .send()
        .await
        .expect("Failed to send get product request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse get product response JSON");

    // Ensure product contains the expected fields
    assert!(body["id"].is_number());
    assert!(body["name"].is_string());
}

#[tokio::test]
async fn test_patch_product() {
    let client = Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "admin",
        "password": "Muzion15"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = login_body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Set Authorization Header
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to create Authorization header"),
    );

    // Step 3: Define payload for updating a product
    let patch_payload = json!({
        "name": "Updated Test Product",
        "price": 120.0
    });

    // Step 4: Send request to patch product (replace 1 with actual product id)
    let response = client
        .patch("http://127.0.0.1:3000/api/admin/product/1")
        .headers(headers)
        .json(&patch_payload)
        .send()
        .await
        .expect("Failed to send patch product request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse patch product response JSON");

    assert_eq!(
        body["message"].as_str(),
        Some("Resource patched successfully.")
    );
}

#[tokio::test]
async fn test_delete_product() {
    let client = Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "admin",
        "password": "Muzion15"
    });

    let login_response = client
        .post("http://127.0.0.1:3000/login")
        .json(&login_payload)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = login_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse login response JSON");

    let token = login_body["token"]
        .as_str()
        .expect("Token not found in login response");

    // Step 2: Set Authorization Header
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Failed to create Authorization header"),
    );

    // Step 3: Send request to delete product (replace 1 with actual product id)
    let response = client
        .delete("http://127.0.0.1:3000/api/admin/product/1")
        .headers(headers)
        .send()
        .await
        .expect("Failed to send delete product request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse delete product response JSON");

    assert_eq!(
        body["message"].as_str(),
        Some("Resource deleted successfully.")
    );
}
