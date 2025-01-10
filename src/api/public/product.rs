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

use crate::entities::product::{self, Entity as ProductEntity};

pub fn product_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/product", get(get_products))
        .route("/product/:id", get(get_product))
        .layer(Extension(db))
}

async fn get_products(
    Query(params): Query<GetProductsQuery>,
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
    let mut half_result = ProductEntity::find().filter(product::Column::IsAvailable.eq(true));
    //.filter(category::Column::IsAvailable.eq(true));

    if Some(true) == params.featured {
        half_result = half_result.filter(product::Column::IsFeatured.eq(true));
    }

    if let Some(min) = params.min {
        half_result = half_result.filter(product::Column::Price.gte(min));
    }

    if let Some(max) = params.max {
        half_result = half_result.filter(product::Column::Price.lte(max));
    }

    let result = half_result.all(&txn).await;
    match result {
        Ok(products) => {
            let response: Vec<PublicProductResponse> = products
                .into_iter()
                .map(|prod| PublicProductResponse::new(prod))
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

async fn get_product(
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
    let result = ProductEntity::find_by_id(id)
        .filter(product::Column::IsAvailable.eq(true))
        .one(&txn)
        .await;
    match result {
        Ok(Some(prod)) => (StatusCode::OK, Json(PublicProductResponse::new(prod))).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("No product with {} id was found.", id)
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
struct GetProductsQuery {
    featured: Option<bool>,
    min: Option<f32>,
    max: Option<f32>,
}

#[derive(Serialize)]
struct PublicProductResponse {
    id: i32,
    name: String,
    price: f32,
    description: String,
    image_id: i32,
    category_id: i32,
}

impl PublicProductResponse {
    fn new(value: product::Model) -> PublicProductResponse {
        PublicProductResponse {
            id: value.id,
            name: value.name,
            price: value.price,
            description: value.description,
            image_id: value.image_id,
            category_id: value.category_id,
        }
    }
}
