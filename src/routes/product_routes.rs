use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    middleware::{self},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::entities::{category, image, product::{self, Entity as ProductEntity}, user};
use crate::middleware::auth::jwt_middleware;

//ROUTERS
pub async fn product_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route("/product", get(get_products))
        .route("/product/:id", get(get_product))
        .layer(Extension(db))
}

pub async fn admin_product_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route("/product", post(create_product))
        .route(
            "/product/:id",
            get(admin_get_product)
                .patch(patch_product)
                .delete(delete_product),
        )
        .layer(Extension(db))
        .layer(middleware::from_fn(jwt_middleware))
}

//ROUTES
async fn create_product(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<CreateProduct>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_product()` with payload: \n>{:?}",
        payload.clone()
    );
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => match user::Entity::find_by_id(payload.image_id).one(&txn).await {
            Ok(Some(_)) => {
                let new_product = product::ActiveModel {
                    name: Set(payload.name),
                    price: Set(payload.price),
                    description: Set(payload.description),
                    image_id: Set(payload.image_id),
                    category_id: Set(payload.category_id),
                    is_featured: Set(payload.is_featured.unwrap_or_default()),
                    is_available: Set(payload.is_available.unwrap_or_default()),
                    ..Default::default()
                };

                match product::Entity::insert(new_product).exec(&txn).await {
                    Ok(_) => (
                        StatusCode::CREATED,
                        Json(json!({
                            "message": "Product created successfully"
                        })),
                    ),
                    Err(err) => {
                        println!("Error: {:?}", err);
                        let _ = txn.rollback().await;
                        (
                            StatusCode::CONFLICT,
                            Json(json!({
                                "error": "Product already exists"
                            })),
                        )
                    }
                }
            }
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
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

async fn get_products(
    Query(params): Query<GetProductsQuery>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let mut half_result =
                ProductEntity::find().filter(product::Column::IsAvailable.eq(true));

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

async fn get_product(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ProductEntity::find_by_id(id)
                .filter(product::Column::IsAvailable.eq(true))
                .one(&txn)
                .await;
            match result {
                Ok(Some(prod)) => {
                    (StatusCode::OK, Json(PublicProductResponse::new(prod))).into_response()
                }
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
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

async fn admin_get_product(
    Query(params): Query<GetProductQuery>,
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ProductEntity::find_by_id(id)
                .one(&txn)
                .await;

            match result {
                Ok(Some(prod)) => match params.full {
                    Some(true) => (StatusCode::OK, Json(prod)).into_response(),
                    Some(false) | None => {
                        (StatusCode::OK, Json(PublicProductResponse::new(prod))).into_response()
                    }
                },
                Ok(None) => (
                    StatusCode::BAD_REQUEST,
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
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

async fn patch_product(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<PatchProductPayload>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ProductEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(product)) => {
                    let mut product: product::ActiveModel = product.into();

                    if let Some(name) = payload.name {
                        product.name = Set(name);
                    }

                    if let Some(price) = payload.price {
                        product.price = Set(price);
                    }

                    if let Some(description) = payload.description {
                        product.description = Set(description);
                    }

                    if let Some(image_id) = payload.image_id {
                        match image::Entity::find_by_id(image_id).one(&txn).await {
                            Ok(_) => product.image_id = Set(image_id),
                            Err(_) => {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!({
                                        "error": format!("No image with {image_id} id was found")
                                    })),
                                )
                                    .into_response();
                            }
                        }
                    }

                    if let Some(category_id) = payload.category_id {
                        match category::Entity::find_by_id(category_id).one(&txn).await {
                            Ok(_) => product.category_id = Set(category_id),
                            Err(_) => {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!({
                                        "error": format!("No category with {category_id} id was found")
                                    })),
                                )
                                    .into_response();
                            }
                        }
                    }

                    if let Some(is_featured) = payload.is_featured {
                        product.is_featured = Set(is_featured);
                    }

                    if let Some(is_available) = payload.is_available {
                        product.is_available = Set(is_available);
                    }

                    let result = product.update(&txn).await;
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

async fn delete_product(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => {
            let result = ProductEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(product)) => {
                    let product: product::ActiveModel = product.into();
                    let result = product.delete(&txn).await;
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

//Structs
#[derive(Deserialize, Clone, Debug)]
struct CreateProduct {
    name: String,
    price: f32,
    description: String,
    image_id: i32,
    category_id: i32,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Deserialize)]
struct GetProductsQuery {
    featured: Option<bool>,
    min: Option<f32>,
    max: Option<f32>,
}

#[derive(Deserialize)]
struct GetProductQuery {
    full: Option<bool>,
}

#[derive(Deserialize)]
struct PatchProductPayload {
    name: Option<String>,
    price: Option<f32>,
    description: Option<String>,
    image_id: Option<i32>,
    category_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
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
            category_id: value.category_id
        }
    }
}