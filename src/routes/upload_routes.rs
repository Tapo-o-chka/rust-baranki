use axum::extract::Query;
use axum::routing::get;
use axum::{
    extract::{Extension, Multipart, Path},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::Response,
    routing::{patch, post},
    Json, Router,
};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs as tokio_fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use validator::Validate;

use crate::entities::image::FileExtension;
use crate::entities::{image, image::Entity as ImageEntity, user::Role};
use crate::middleware::{
    auth::auth_middleware,
    logging::{to_response, ApiError},
};

//Routers
pub fn public_image_router() -> Router {
    Router::new().route("/image/:id", get(print_image))
}

pub fn upload_routes() -> Router {
    Router::new()
        .route("/image", post(upload).get(get_images))
        .route("/image/:id", patch(patch_image).delete(delete_image))
        .layer(middleware::from_fn_with_state(Role::Admin, auth_middleware))
}

//Routes
pub async fn print_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    let path = match ImageEntity::find_by_id(id).one(&txn).await {
        Ok(Some(model)) => {
            "./uploads/".to_owned() + &model.path_name + "." + &model.extension.to_string()
        }
        Ok(None) => {
            let tmp = format!("Image not found with {id} id");
            return to_response(
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": tmp
                    })),
                ),
                Err(ApiError::General(tmp)),
            );
        }
        Err(err) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::DbError(err.to_string())),
            );
        }
    };

    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) => {
            return to_response(
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": "Not found"
                    })),
                ),
                Err(ApiError::General(err.to_string())),
            )
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

    to_response((headers, body), Ok(()))
}

async fn upload(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    mut multipart: Multipart,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    loop {
        match multipart.next_field().await.unwrap_or(None) {
            Some(field) => {
                let content_type = match field.content_type() {
                    Some(content_type) => content_type.to_owned(),
                    None => {
                        let tmp = "Content type is not set.";
                        return to_response(
                            (StatusCode::BAD_REQUEST, Json(json!({"error": tmp}))),
                            Err(ApiError::General(tmp.to_string())),
                        );
                    }
                };

                let file_extension = match allowed_content_types().get(content_type.as_str()) {
                    Some(&ext) => ext.to_owned(),
                    None => {
                        let tmp = "Unsupported content type.";
                        return to_response(
                            (StatusCode::BAD_REQUEST, Json(json!({"error": tmp}))),
                            Err(ApiError::General(tmp.to_string())),
                        );
                    }
                };

                let file_name = match field.name() {
                    Some(name) => name.to_owned(),
                    None => {
                        let tmp = "File name is not set.";
                        return to_response(
                            (StatusCode::BAD_REQUEST, Json(json!({"error": tmp}))),
                            Err(ApiError::General(tmp.to_string())),
                        );
                    }
                };

                if !FILE_NAME_REGEX.is_match(&file_name) {
                    return to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Invalid file name. It should contain only Latin letters, numbers, '-', or '_'."
                            })),
                        ),
                        Err(ApiError::General("Regex match failed".to_string())),
                    );
                }

                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(err) => {
                        return to_response(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Failed to read file bytes."
                                })),
                            ),
                            Err(ApiError::General(format!("Multipart error: {err}"))),
                        );
                    }
                };
                if data.len() > get_file_size_limit() {
                    let tmp = "Payload too large";
                    return to_response(
                        (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            Json(json!({
                                "error": tmp
                            })),
                        ),
                        Err(ApiError::General(tmp.to_string())),
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
                                Ok(_) => to_response(
                                    (
                                        StatusCode::CREATED,
                                        Json(json!({
                                            "message": "File uploaded successfully."
                                        })),
                                    ),
                                    Ok(()),
                                ),
                                Err(err) => to_response(
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Internal server error."
                                        })),
                                    ),
                                    Err(ApiError::DbError(err.to_string())),
                                ),
                            },
                            Err(err) => {
                                let _ = txn.rollback().await;
                                to_response(
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Failed to upload file to the server"
                                        })),
                                    ),
                                    Err(ApiError::DbError(err.to_string())),
                                )
                            }
                        };
                    }
                    Err(err) => {
                        let _ = txn.rollback().await;
                        return to_response(
                            (
                                StatusCode::CONFLICT,
                                Json(json!({
                                    "error": "Image already exists"
                                })),
                            ),
                            Err(ApiError::DbError(err.to_string())),
                        );
                    }
                }
            }
            None => {
                let tmp = "Dont know what went wrong";
                return to_response(
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": tmp
                        })),
                    ),
                    Err(ApiError::General(tmp.to_string())),
                );
            }
        }
    }
}

async fn get_images(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(query): Query<ImagesQuery>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    let filter = if let Some(query) = query.query {
        let mut query_condition =
            Condition::any().add(image::Column::FileName.contains(query.clone()));
        let id_search = query.parse::<u32>().ok();
        if let Some(id) = id_search {
            query_condition = query_condition.add(image::Column::Id.eq(id));
        };

        query_condition
    } else {
        Condition::any()
    }; //will it work?

    let result = ImageEntity::find().filter(filter).all(&txn).await;
    match result {
        Ok(images) => to_response((StatusCode::OK, Json(images)), Ok(())),
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn patch_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchImagePayload>,
) -> Response {
    if let Some(err) = payload.validate().err() {
        return to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid file name. It should contain only Latin letters, numbers, '-', or '_'."
                })),
            ),
            Err(ApiError::ValidationFail(err.to_string())),
        );
    }

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
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
                Ok(_) => match txn.commit().await {
                    Ok(_) => to_response(
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource patched successfully."
                            })),
                        ),
                        Ok(()),
                    ),
                    Err(err) => to_response(
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Internal server error"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    ),
                },
                Err(err) => {
                    //DB Failed / unique constraint
                    let _ = txn.rollback().await;
                    to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to patch this resource"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    )
                }
            }
        }
        Ok(None) => {
            let tmp = format!("No image with {} id was found.", id);
            to_response(
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": tmp
                    })),
                ),
                Err(ApiError::General(tmp)),
            )
        }
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn delete_image(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    match ImageEntity::find_by_id(id).one(&txn).await {
        Ok(Some(image)) => {
            let file_path = image.path_name.clone();

            let image_active: image::ActiveModel = image.into();
            match image_active.delete(&txn).await {
                Ok(_) => {
                    match tokio_fs::remove_file(format!("./uploads/{}.jpg", &file_path)).await {
                        Ok(_) => match txn.commit().await {
                            Ok(_) => to_response(
                                (
                                    StatusCode::OK,
                                    Json(json!({
                                        "message": "Resource deleted successfully."
                                    })),
                                ),
                                Ok(()),
                            ),
                            Err(err) => to_response(
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "error": "Internal server error"
                                    })),
                                ),
                                Err(ApiError::DbError(err.to_string())),
                            ),
                        },
                        Err(err) => {
                            let _ = txn.rollback().await;
                            to_response(
                                (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!({
                                        "error": "Failed to delete this resource"
                                    })),
                                ),
                                Err(ApiError::DbError(err.to_string())),
                            )
                        }
                    }
                }
                Err(err) => {
                    let _ = txn.rollback().await;
                    to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to delete this resource"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    )
                }
            }
        }
        Ok(None) => {
            let tmp = format!("No image with id {} was found.", id);
            to_response(
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("No image with id {} was found.", id)
                    })),
                ),
                Err(ApiError::General(tmp)),
            )
        }
        Err(err) => {
            let _ = txn.rollback().await;
            to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Failed to fetch image from database"
                    })),
                ),
                Err(ApiError::DbError(err.to_string())),
            )
        }
    }
}

//structs
#[derive(Deserialize, Validate)]
struct PatchImagePayload {
    #[validate(regex(path = *FILE_NAME_REGEX))]
    file_name: String,
}

#[derive(Deserialize)]
struct ImagesQuery {
    query: Option<String>,
}

//utils
fn allowed_content_types() -> HashMap<&'static str, FileExtension> {
    HashMap::from([
        ("image/jpeg", FileExtension::JPG),
        ("image/png", FileExtension::PNG),
    ])
}

static FILE_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,25}$").unwrap());

fn get_file_size_limit() -> usize {
    dotenv().ok();
    std::env::var("FILE_SIZE_LIMIT")
        .expect("FILE_SIZE_LIMIT not found in .env file")
        .parse::<usize>()
        .expect("Failed to parse FILE_SIZE_LIMIT")
}
