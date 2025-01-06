use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
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

use crate::entities::{category, category::Entity as CategoryEntity, image, user, user::Role};
use crate::middleware::auth::{auth_middleware, AuthState};

//ROUTERS
pub async fn category_routes(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/category", get(get_categories))
        .route("/category/:id", get(get_category))
        .layer(Extension(db))
}

pub async fn admin_category_routes(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/category", post(create_category))
        .route(
            "/category/:id",
            get(admin_get_category)
                .patch(patch_category)
                .delete(delete_category),
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

//ROUTES
async fn create_category(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateCategory>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_category()` with payload: \n>{:?}",
        payload.clone()
    );

    match db.begin().await {
        Ok(txn) => match user::Entity::find_by_id(payload.image_id).one(&txn).await {
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

async fn get_categories(
    Query(params): Query<GetCategoriesQuery>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {

    match db.begin().await {
        Ok(txn) => {
            let mut half_result =
                CategoryEntity::find().filter(category::Column::IsAvailable.eq(true));

            if Some(true) == params.featured {
                half_result = half_result.filter(category::Column::IsFeatured.eq(true));
            }

            let result = half_result.all(&txn).await;
            match result {
                Ok(categories) => {
                    let response: Vec<PublicCategoryResponse> = categories
                        .into_iter()
                        .map(|categ| PublicCategoryResponse::new(categ))
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

async fn get_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    match db.begin().await {
        Ok(txn) => {
            let result = CategoryEntity::find_by_id(id)
                .filter(category::Column::IsAvailable.eq(true))
                .one(&txn)
                .await;
            match result {
                Ok(Some(categor)) => {
                    (StatusCode::OK, Json(PublicCategoryResponse::new(categor))).into_response()
                }
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

async fn admin_get_category(
    Query(params): Query<GetCategoryQuery>,
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {

    match db.begin().await {
        Ok(txn) => {
            let result = CategoryEntity::find_by_id(id)
                .one(&txn)
                .await;
            match result {
                Ok(Some(categor)) => match params.full {
                    Some(true) => (StatusCode::OK, Json(categor)).into_response(),
                    Some(false) | None => {
                        (StatusCode::OK, Json(PublicCategoryResponse::new(categor))).into_response()
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

async fn patch_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchCategoryPayload>,
) -> impl IntoResponse {

    match db.begin().await {
        Ok(txn) => {
            let result = CategoryEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(category)) => {
                    let mut category: category::ActiveModel = category.into();

                    if let Some(name) = payload.name {
                        category.name = Set(name);
                    }
                    if let Some(image_id) = payload.image_id {
                        match image::Entity::find_by_id(image_id).one(&txn).await {
                            Ok(_) => category.image_id = Set(image_id),
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

                    if let Some(is_featured) = payload.is_featured {
                        category.is_featured = Set(is_featured);
                    }

                    if let Some(is_available) = payload.is_available {
                        category.is_available = Set(is_available);
                    }

                    let result = category.update(&txn).await;
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

async fn delete_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {

    match db.begin().await {
        Ok(txn) => {
            let result = CategoryEntity::find_by_id(id).one(&txn).await;
            match result {
                Ok(Some(category)) => {
                    let category: category::ActiveModel = category.into();
                    let result = category.delete(&txn).await;
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

//Struct
#[derive(Deserialize, Clone, Debug)]
struct CreateCategory {
    name: String,
    image_id: i32,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Deserialize)]
struct GetCategoryQuery {
    full: Option<bool>,
}

#[derive(Deserialize)]
struct GetCategoriesQuery {
    featured: Option<bool>,
}

#[derive(Deserialize)]
struct PatchCategoryPayload {
    name: Option<String>,
    image_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Serialize)]
struct PublicCategoryResponse {
    id: i32,
    name: String,
    image_id: i32,
}

impl PublicCategoryResponse {
    fn new(value: category::Model) -> PublicCategoryResponse {
        PublicCategoryResponse {
            id: value.id,
            name: value.name,
            image_id: value.image_id,
        }
    }
}
