use std::sync::Arc;
use tokio::sync::Mutex;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use axum::{
    extract::Extension,
    extract::Multipart,
    http::StatusCode,
    middleware::{self},
    response::IntoResponse,
    routing::post,
    Json, Router,
};

const FILE_SIZE_LIMIT: usize = 8 * 1024 * 1024 * 8;

use crate::{entities::{category, user}, middleware::auth::jwt_middleware};

pub async fn upload_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route(
            "/image",
            post(upload),
        )
        .layer(Extension(db))
}

fn allowed_content_types() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("image/jpeg", "jpg"),
        ("image/png", "png"),
    ])
}

async fn upload(mut multipart: Multipart) -> impl IntoResponse {
    println!("Called");
    loop {
        match multipart.next_field().await.unwrap_or(None) {
            Some(field) => {
                let content_type = match field.content_type() {
                    Some(content_type) => content_type.to_owned(),
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Content type is not set."})),
                        );
                    }
                };
        
                let file_extension = match allowed_content_types().get(content_type.as_str()) {
                    Some(&ext) => ext.to_owned(),
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Unsupported content type."})),
                        );
                    }
                };
                
                let file_name = match field.name() {
                    Some(name) => name.to_owned(),
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "File name is not set."
                            }))
                        );
                    }
                };
        
                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(_) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Failed to read file bytes."
                            }))
                        );
                    }
                };
                if data.len() > FILE_SIZE_LIMIT {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({
                            "error": "Payload too large."
                        }))
                    );
                } 
                
                print!("{}", format!("/workspaces/rust-baranki/uploads/{}.{}\n", file_name, file_extension));
                
                return match std::fs::write(format!("/workspaces/rust-baranki/uploads/{}.{}", file_name, file_extension), data) {
                    Ok(_) => (
                        StatusCode::CREATED,
                        Json(json!({
                            "message": "File uploaded successfully."
                        }))
                    ),
                    Err(err) => {
                        println!("> Error: 'Failed to upload file to the server'.\n> Exactly: {:?}", err);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Failed to upload file to the server"
                            }))
                        )
                    }
                };
            },
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "idk what went wrong"
                    })),
                )
            }
        }
    }
}