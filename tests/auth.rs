use reqwest::{header, Client};
use tokio;

// Test if the server is running and responds to a health check
#[tokio::test]
async fn test_health_check() {
    let client = Client::new();

    let response = client
        .get("http://127.0.0.1:3000/")
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success(), "Health check failed");
    let body = response.text().await.expect("Failed to read response body");
    println!("{:?}", body);
}

//auth testing
#[tokio::test]
async fn test_create_user() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "JohnDoe",
        "password": "Secret15"
    });

    let response = client
        .post("http://127.0.0.1:3000/register")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    println!("{:?}", body);
}

#[tokio::test]
async fn test_login() {
    let client = Client::new();

    let payload = serde_json::json!({
        "username": "user",
        "password": "Secret15"
    });

    let response = client
        .post("http://127.0.0.1:3000/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    println!("Login!");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response JSON");
    let binding = body["token"].clone();

    if let Some(token) = binding.as_str() {
        //TESTING WITH RIGHT HEADER
        let mut headers = header::HeaderMap::new();
        println!("token {:?}", token);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/profile")
            .headers(headers)
            .send()
            .await
            .expect("Failed to send request to protected url");
        println!("Protected!");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response JSON");
        println!("{:?}", body);

        //TESTING WITH WRONG HEADER
        let mut wrong_headers = header::HeaderMap::new();
        wrong_headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}d", token))
                .expect("Failed to insert header"),
        );
        let response = client
            .get("http://127.0.0.1:3000/api/profile")
            .headers(wrong_headers)
            .send()
            .await
            .expect("Failed to send request to protected url");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }
    //TESING WITHOUT HEADER
    let response = client
        .get("http://127.0.0.1:3000/api/profile")
        .send()
        .await
        .expect("Failed to send request to protected url");

    println!("{:?}", response);
}

