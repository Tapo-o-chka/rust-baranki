use reqwest::header;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::json;
use tokio;

#[tokio::test]
async fn test_get_cart() {
    let client = reqwest::Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "user",
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

    // Step 3: Send request to get cart
    let get_response = client
        .get("http://127.0.0.1:3000/api/cart")
        .headers(headers)
        .send()
        .await
        .expect("Failed to send get cart request");

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = get_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse get cart response JSON");

    // Step 4: Assert expected response
    assert!(get_body.is_array());
}

#[tokio::test]
async fn test_add_product_to_cart() {
    let client = reqwest::Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "user",
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

    // Step 3: Define payload for adding product to cart
    let add_product_payload = json!(AddProduct {
        product_id: 1,
        quantity: 2
    });

    // Step 4: Send request to add product to cart
    let add_response = client
        .post("http://127.0.0.1:3000/api/cart")
        .headers(headers)
        .json(&add_product_payload)
        .send()
        .await
        .expect("Failed to send add product request");

    assert_eq!(add_response.status(), StatusCode::CREATED);

    let add_body = add_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse add product response JSON");

    assert_eq!(
        add_body["message"].as_str(),
        Some("Added successfully")
    );
}

#[tokio::test]
async fn test_remove_product_from_cart() {
    let client = reqwest::Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "user",
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

    // Step 3: Send request to remove product from cart
    let remove_response = client
        .delete("http://127.0.0.1:3000/api/cart/1")
        .headers(headers)
        .send()
        .await
        .expect("Failed to send remove product request");

    assert_eq!(remove_response.status(), StatusCode::OK);

    let remove_body = remove_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse remove product response JSON");

    assert_eq!(
        remove_body["message"].as_str(),
        Some("Resource deleted successfully")
    );
}

#[tokio::test]
async fn test_patch_cart_entry() {
    let client = reqwest::Client::new();

    // Step 1: Authenticate and retrieve token
    let login_payload = json!({
        "username": "user",
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

    // Step 3: Define payload for patching cart entry
    let patch_payload = json!({
        "quantity": 5
    });

    // Step 4: Send request to patch cart entry
    let patch_response = client
        .patch("http://127.0.0.1:3000/api/cart/1")
        .headers(headers)
        .json(&patch_payload)
        .send()
        .await
        .expect("Failed to send patch cart request");

    assert_eq!(patch_response.status(), StatusCode::OK);

    let patch_body = patch_response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse patch cart response JSON");

    assert_eq!(
        patch_body["message"].as_str(),
        Some("Resource patched successfully")
    );
}

#[derive(Serialize, Debug)]
struct AddProduct {
    product_id: i32,
    quantity: u32, //maybe u16 is enough...
}
