use axum::routing::get;
use axum::{
    extract::{Extension, Multipart, Path},
    http::StatusCode,
    //middleware::{self},
    response::IntoResponse,
    routing::post,
    Json,
    Router,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const FILE_SIZE_LIMIT: usize = 8 * 1024 * 1024 * 8;

use crate::entities::image;
//use crate::middleware::auth::jwt_middleware;
use crate::entities::image::Entity as ImageEntity;

pub async fn upload_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route("/image", post(upload).get(get_images))
        .route(
            "/image/:id",
            get(get_image).patch(patch_image).delete(delete_image),
        )
        .layer(Extension(db))
}

fn allowed_content_types() -> HashMap<&'static str, &'static str> {
    HashMap::from([("image/jpeg", "jpg"), ("image/png", "png")])
}

async fn upload(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    println!("Called");
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => loop {
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
                                })),
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
                                })),
                            );
                        }
                    };
                    if data.len() > FILE_SIZE_LIMIT {
                        return (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            Json(json!({
                                "error": "Payload too large."
                            })),
                        );
                    }

                    let id = Uuid::new_v4().to_string();
                    let new_image = image::ActiveModel {
                        file_name: Set(file_name.clone()),
                        path_name: Set(id.clone()),
                        ..Default::default()
                    };

                    match ImageEntity::insert(new_image).exec(&txn).await {
                        Ok(_) => {
                            return match std::fs::write(
                                format!(
                                    "/workspaces/rust-baranki/uploads/{}.{}",
                                    id, file_extension
                                ),
                                data,
                            ) {
                                Ok(_) => match txn.commit().await {
                                    Ok(_) => (
                                        StatusCode::CREATED,
                                        Json(json!({
                                            "message": "File uploaded successfully."
                                        })),
                                    ),
                                    Err(_) => (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Internal server error."
                                        })),
                                    ),
                                },
                                Err(err) => {
                                    println!("> Error: 'Failed to upload file to the server'.\n> Exactly: {:?}", err);
                                    let _ = txn.rollback().await;
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Failed to upload file to the server"
                                        })),
                                    )
                                }
                            };
                        }
                        Err(err) => {
                            println!("Error: {:?}", err);
                            let _ = txn.rollback().await;
                            return (
                                StatusCode::CONFLICT,
                                Json(json!({
                                    "error": "Image already exists"
                                })),
                            );
                        }
                    }
                }
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "idk what went wrong"
                        })),
                    )
                }
            }
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

async fn get_images(Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ImageEntity::find().all(&txn).await;
            match result {
                Ok(images) => {
                    let response: Vec<ImageResponse> = images
                        .into_iter()
                        .map(|img| ImageResponse {
                            id: img.id,
                            file_name: img.file_name,
                        })
                        .collect();
                    return (StatusCode::OK, Json(response)).into_response();
                }
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Internal server error."
                        })),
                    )
                        .into_response();
                }
            }
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    }
}

async fn get_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ImageEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(image)) => {
                    (StatusCode::OK, Json(ImageResponse::new(image))).into_response()
                }
                Ok(None) => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("No image with {} id was found.", id)
                    })),
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error."
                    })),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

async fn patch_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<PatchImagePayload>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ImageEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(image)) => {
                    let mut image: image::ActiveModel = image.into();
                    image.file_name = Set(payload.file_name);
                    let result = image.update(&txn).await;
                    match result {
                        Ok(new_model) => {
                            let _ = txn.commit().await;
                            println!("New model: {:?}", new_model);
                            (
                                StatusCode::OK,
                                Json(json!({
                                    "message": "Resource patched successfully."
                                })),
                            )
                                .into_response()
                        }
                        Err(_) => {
                            //DB Failed / unique constraint
                            let _ = txn.rollback().await;
                            (
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "error": "Failed to patch this resource"
                                })),
                            )
                                .into_response()
                        }
                    }
                }
                Ok(None) => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("No image with {} id was found.", id)
                    })),
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error."
                    })),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

async fn delete_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ImageEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(image)) => {
                    let image: image::ActiveModel = image.into();
                    let result = image.delete(&txn).await;
                    match result {
                        Ok(new_model) => {
                            let _ = txn.commit().await;
                            println!("New model: {:?}", new_model);
                            (
                                StatusCode::OK,
                                Json(json!({
                                    "message": "Resource deleted successfully."
                                })),
                            )
                                .into_response()
                        }
                        Err(_) => {
                            //DB Failed / unique constraint
                            let _ = txn.rollback().await;
                            (
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "error": "Failed to delete this resource"
                                })),
                            )
                                .into_response()
                        }
                    }
                }
                Ok(None) => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("No image with {} id was found.", id)
                    })),
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error."
                    })),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

//structs
#[derive(Serialize)]
struct ImageResponse {
    id: i32,
    file_name: String,
}

#[derive(Deserialize)]
struct PatchImagePayload {
    file_name: String,
}

impl ImageResponse {
    fn new(value: image::Model) -> ImageResponse {
        ImageResponse {
            id: value.id,
            file_name: value.file_name,
        }
    }
}
