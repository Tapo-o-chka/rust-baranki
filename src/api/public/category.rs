use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::{category, category::Entity as CategoryEntity};

pub fn category_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/category", get(get_categories))
        .route("/category/:id", get(get_category))
        .layer(Extension(db))
}

async fn get_categories(
    Query(params): Query<GetCategoriesQuery>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    println!("Called get categories");
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
    let mut half_result = CategoryEntity::find().filter(category::Column::IsAvailable.eq(true));

    if Some(true) == params.featured {
        half_result = half_result.filter(category::Column::IsFeatured.eq(true));
    }

    let result = half_result.all(&txn).await;
    match result {
        Ok(categories) => {
            let response: Vec<CategoryResponse> = categories
                .into_iter()
                .map(|categ| CategoryResponse::new(categ))
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

async fn get_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    println!("->> Called `create_category()`",);
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

    let result = CategoryEntity::find_by_id(id)
        .filter(category::Column::IsAvailable.eq(true))
        .one(&txn)
        .await;
    match result {
        Ok(Some(categor)) => (StatusCode::OK, Json(CategoryResponse::new(categor))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("No category with {} id was found.", id)
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

#[derive(Deserialize)]
struct GetCategoriesQuery {
    featured: Option<bool>,
}

#[derive(Serialize)]
struct CategoryResponse {
    id: i32,
    name: String,
    image_id: Option<i32>,
}

impl CategoryResponse {
    fn new(value: category::Model) -> CategoryResponse {
        CategoryResponse {
            id: value.id,
            name: value.name,
            image_id: value.image_id,
        }
    }
}
