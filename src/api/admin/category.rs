use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::{category, category::Entity as CategoryEntity, image};

//ROUTERS
pub fn admin_category_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/category", post(create_category))
        .route(
            "/category/:id",
            get(admin_get_category)
                .patch(patch_category)
                .delete(delete_category),
        )
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

    if let Some(image_id) = payload.image_id {
        match image::Entity::find_by_id(image_id).one(&txn).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": format!("Image with id {} not found", image_id)
                    })),
                );
            }
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                );
            }
        }
    }

    //IF MODEL CHANGES DEFAULT VALUE -> NEED TO CHANGE HERE TOO
    let new_category = category::ActiveModel {
        name: Set(payload.name),
        image_id: Set(payload.image_id),
        is_featured: Set(payload.is_featured.unwrap_or_else(|| false)), //bad spot .unwrap_or_default() makes it not sea_orm default, but rust default to false
        is_available: Set(payload.is_available.unwrap_or_else(|| true)), //bad spot .unwrap_or_default() makes it not sea_orm default, but rust default to false
        ..Default::default()
    };

    match category::Entity::insert(new_category).exec(&txn).await {
        Ok(_) => match txn.commit().await {
            Ok(_) => (
                StatusCode::CREATED,
                Json(json!({
                    "message": "Category created successfully"
                })),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            ),
        },
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

async fn admin_get_category(
    Query(params): Query<GetCategoryQuery>,
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
    let result = CategoryEntity::find_by_id(id).one(&txn).await;
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

async fn patch_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchCategory>,
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
    let result = CategoryEntity::find_by_id(id).one(&txn).await;
    match result {
        Ok(Some(category)) => {
            let mut category: category::ActiveModel = category.into();

            if let Some(name) = payload.name {
                category.name = Set(name);
            }
            if let Some(image_id) = payload.image_id {
                match image::Entity::find_by_id(image_id).one(&txn).await {
                    Ok(_) => category.image_id = Set(Some(image_id)),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": format!("No image with {image_id} id was found")
                            })),
                        );
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

async fn delete_category(
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
            );
        }
    };
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

//Struct
#[derive(Deserialize, Clone, Debug)]
struct CreateCategory {
    name: String,
    image_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Deserialize)]
struct GetCategoryQuery {
    full: Option<bool>,
}

#[derive(Deserialize)]
struct PatchCategory {
    name: Option<String>,
    image_id: Option<i32>,
    is_featured: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Serialize)]
struct PublicCategoryResponse {
    id: i32,
    name: String,
    image_id: Option<i32>,
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
