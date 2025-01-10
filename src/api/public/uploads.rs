use axum::routing::get;
use axum::{
    extract::{Extension, Path},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json, Router,
};
use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait};
use serde_json::json;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::entities::image::Entity as ImageEntity;

pub fn uploads_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/image/:id", get(print_image))
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
