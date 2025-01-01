use std::sync::Arc;
use tokio::sync::Mutex;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::json;
use axum::{
    extract::Extension,
    http::StatusCode,
    middleware::{self},
    response::IntoResponse,
    routing::post,
    Json, Router,
};

use crate::{entities::{category, user}, middleware::auth::jwt_middleware};

pub async fn category_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route(
            "/category",
            post(create_category),
        )
        .layer(middleware::from_fn(jwt_middleware))
        .layer(Extension(db))
}

//ROUTES
async fn create_category(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<CreateCategory>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_category()` with payload: \n>{:?}",
        payload.clone()
    );
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            match user::Entity::find().filter(crate::entities::image::Column::Id.eq(payload.image_id)).one(&txn).await {
                Ok(Some(_)) => {
                    let new_category = category::ActiveModel {
                        name: Set(payload.name),
                        image_id: Set(payload.image_id),
                        is_featured: Set(payload.is_featured.unwrap_or_default()),
                        is_available: Set(payload.is_available.unwrap_or_default()),
                        ..Default::default()
                    };
                
                    match category::Entity::insert(new_category).exec(&txn).await {
                        Ok(_) => (
                            StatusCode::CREATED,
                            Json(json!({
                                "message": "Category created successfully"
                            })),
                        ),
                        Err(err) => {
                            println!("Error: {:?}", err);
                            let _ = txn.rollback().await;
                            (
                                StatusCode::CONFLICT,
                                Json(json!({
                                    "error": "Category already exists"
                                })),
                            )
                        }
                    }
                },
                Ok(None) => {
                    let _ = txn.rollback().await;
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "error": format!("Image with id {} not found", payload.image_id)
                        })),
                    )
                }
                Err(_) => {
                    let _ = txn.rollback().await;
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Internal server error"
                        })),
                    )
                }
            }
        },
        Err(_) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
        }
    }
}

//Struct
#[derive(Deserialize, Clone, Debug)]
struct CreateCategory {
    name: String,
    image_id: i32,
    is_featured: Option<bool>,
    is_available: Option<bool>,    
}