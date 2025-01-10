use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio;

#[tokio::test]
async fn test_category() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "user",
        "password": "12345"
    });

    let response = client
        .post("http://127.0.0.1:3000/api/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        
        //TESTING ON NON EMPTY ARRAY
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/category")
            .headers(headers)
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response JSON");
        assert_eq!(body, serde_json::Value::Array(vec![]));
    }
}

#[tokio::test]
async fn test_get_category() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "user",
        "password": "12345"
    });
    
    let response = client
        .post("http://127.0.0.1:3000/api/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/category/1")
            .headers(headers)
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response JSON");
        println!("Body:{body}");
    }
}

#[tokio::test]
async fn test_create_category() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "admin",
        "password": "12345"
    });
    
    let response = client
        .post("http://127.0.0.1:3000/api/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        
        //TESTING ON NON EMPTY ARRAY
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );

        let payload = CreateCategory {
            name: "Круглые".to_string(),
            image_id: None,
            is_featured: None,
            is_available: None
        };

        let response = client
            .post("http://127.0.0.1:3000/api/admin/category")
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request to protected url");

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response JSON");
        println!("Body:{body}");
    }
}

#[derive(Serialize, Clone, Debug)]
struct CreateCategory {
    name: String,
    image_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[tokio::test]
async fn test_patch_category() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "admin",
        "password": "12345"
    });
    
    let response = client
        .post("http://127.0.0.1:3000/api/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/category/1")
            .headers(headers.clone())
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_eq!(response.status(), StatusCode::OK);
        let body_1 = response
            .json::<CategoryResponse>()
            .await
            .expect("Failed to parse response JSON");

        let payload = PatchCategory {
            name: Some("Не Круглые".to_string()),
            image_id: None,
            is_featured: None,
            is_available: None
        };

        let response = client
            .patch("http://127.0.0.1:3000/api/admin/category/1")
            .headers(headers.clone())
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request to protected url");

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get("http://127.0.0.1:3000/api/category/1")
            .headers(headers)
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_eq!(response.status(), StatusCode::OK);
        let body_2 = response
            .json::<CategoryResponse>()
            .await
            .expect("Failed to parse response JSON");

        assert_ne!(body_1, body_2)
    }
}

#[derive(Serialize)]
struct PatchCategory {
    name: Option<String>,
    image_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Deserialize, PartialEq, Debug)]
struct CategoryResponse {
    id: i32,
    name: String,
    image_id: Option<i32>,
}

#[tokio::test]
async fn test_delete_category() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "admin",
        "password": "12345"
    });
    
    let response = client
        .post("http://127.0.0.1:3000/api/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/category/1")
            .headers(headers.clone())
            .send()
            .await
            .expect("Failed to send request to protected url");
        let status_code_1 = response.status();
        
        let response = client
            .delete("http://127.0.0.1:3000/api/admin/category/1")
            .headers(headers.clone())
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get("http://127.0.0.1:3000/api/category/1")
            .headers(headers.clone())
            .send()
            .await
            .expect("Failed to send request to protected url");
        assert_ne!(response.status(), status_code_1);
    }
}