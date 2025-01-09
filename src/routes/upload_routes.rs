use axum::routing::get;
use axum::{
    extract::{Extension, Multipart, Path},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs as tokio_fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

const FILE_SIZE_LIMIT: usize = 8 * 1024 * 1024 * 8;

use crate::entities::image::FileExtension;
use crate::entities::{image, image::Entity as ImageEntity, user::Role};
use crate::middleware::auth::{auth_middleware, AuthState};

pub async fn public_image_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/image/:id", get(print_image))
        .layer(Extension(db))
}

pub async fn upload_routes(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/image", post(upload).get(get_images))
        .route(
            "/image/:id",
            get(get_image).patch(patch_image).delete(delete_image),
        )
        .layer(axum::middleware::from_fn_with_state(
            AuthState {
                db: db.clone(),
                role: Role::Admin,
            },
            auth_middleware,
        ))
        .layer(Extension(db))
}

pub async fn print_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            ));
        }
    };

    let path = match ImageEntity::find_by_id(id).one(&txn).await {
        Ok(Some(model)) => {
            "./uploads/".to_owned() + &model.path_name + "." + &model.extension.to_string()
        }
        Ok(None) => {
            println!("havent found this one");
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Not found"
                })),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            ));
        }
    };
    println!("Will panic after that");
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Not found"
                })),
            ))
        }
    };

    let content_type = mime_guess::from_path(&path)
        .first_raw()
        .unwrap_or("application/octet-stream");

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline"),
    );

    Ok((headers, body))
}

fn allowed_content_types() -> HashMap<&'static str, FileExtension> {
    HashMap::from([
        ("image/jpeg", FileExtension::JPG),
        ("image/png", FileExtension::PNG),
    ])
}

async fn upload(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    println!("->> Called `upload()`");
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };
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
                    extension: Set(file_extension),
                    ..Default::default()
                };

                match ImageEntity::insert(new_image).exec(&txn).await {
                    Ok(_) => {
                        return match std::fs::write(
                            format!(
                                "/workspaces/rust-baranki/uploads/{}.{}",
                                id,
                                file_extension.to_string()
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
    }
}

async fn get_images(Extension(db): Extension<Arc<DatabaseConnection>>) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    };

    let result = ImageEntity::find().all(&txn).await;
    match result {
        Ok(images) => {
            let response: Vec<ImageResponse> = images
                .into_iter()
                .map(|img| ImageResponse::new(img))
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

async fn get_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    };

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

async fn patch_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchImagePayload>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

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
                }
            }
        }
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("No image with {} id was found.", id)
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error."
            })),
        ),
    }
}

async fn delete_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
        }
    };
    match ImageEntity::find_by_id(id).one(&txn).await {
        Ok(Some(image)) => {
            let file_path = image.path_name.clone();

            let image_active: image::ActiveModel = image.into();
            match image_active.delete(&txn).await {
                Ok(_) => {
                    match tokio_fs::remove_file(format!("./uploads/{}.jpg", &file_path)).await {
                        Ok(_) => {
                            let _ = txn.commit().await;
                            (
                                StatusCode::OK,
                                Json(json!({
                                    "message": "Resource deleted successfully."
                                })),
                            )
                        }
                        Err(_) => {
                            let _ = txn.rollback().await;
                            (
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "error": "Failed to delete this resource"
                                })),
                            )
                        }
                    }
                }
                Err(_) => {
                    let _ = txn.rollback().await;
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Failed to delete this resource"
                        })),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("No image with id {} was found.", id)
            })),
        ),
        Err(_) => {
            let _ = txn.rollback().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to fetch image from database"
                })),
            )
        }
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
