use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::{category, category::Entity as CategoryEntity, image, user::Role};
use crate::middleware::auth::auth_middleware;

//ROUTERS
pub fn category_routes() -> Router {
    Router::new()
        .route("/category", get(get_categories))
        .route("/category/:id", get(get_category))
}

pub fn admin_category_routes() -> Router {
    Router::new()
        .route("/category", post(create_category).get(admin_get_categories))
        .route(
            "/category/:id",
            patch(patch_category).delete(delete_category),
        )
        .layer(axum::middleware::from_fn_with_state(
            Role::Admin,
            auth_middleware,
        ))
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

async fn admin_get_categories(
    Query(params): Query<AdminCategoriesQuery>,
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

    let mut condition = Condition::all();

    //Filter zone
    if params.only_available.unwrap_or(false) {
        condition = condition.add(category::Column::IsAvailable.eq(true));
    }
    if params.only_featured.unwrap_or(false) {
        condition = condition.add(category::Column::IsFeatured.eq(true))
    }

    //Sorting zone
    let order = match params.order.as_deref() {
        Some("desc") => sea_orm::Order::Desc,
        _ => sea_orm::Order::Asc,
    };

    let sort_column = match params.sort_by.as_deref() {
        Some("name") => category::Column::Name,
        Some("image_id") => category::Column::ImageId,
        Some("is_available") => category::Column::IsAvailable,
        Some("is_featured") => category::Column::IsFeatured,
        _ => category::Column::Id,
    };

    //Pagination zone
    let page: u64 = params.page.unwrap_or(1);
    let page_size: u64 = params.page_size.unwrap_or(10);

    let mut half_items = category::Entity::find();

    //Adding query
    if let Some(query) = params.query {
        let mut query_condition = Condition::any().add(category::Column::Name.contains(query.clone()));
        let id_search = query.parse::<u32>().ok();
        if let Some(id) = id_search {
            query_condition = query_condition.add(category::Column::Id.eq(id));
        }

        half_items = half_items.filter(query_condition); //ahh adding filter after column definitions and ordering, hate that
    }

    //Just put it in the same variable!!!
    let items = half_items
        .filter(condition)
        .order_by(sort_column, order)
        .limit(page_size)
        .offset((page - 1) * page_size)
        .all(&txn)
        .await
        .unwrap_or_else(|_| vec![]);

    Json(items).into_response()
}

async fn get_category(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    println!("->> Called `get_category()`",);
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
        Ok(Some(categor)) => {
            (StatusCode::OK, Json(PublicCategoryResponse::new(categor))).into_response()
        }
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
struct GetCategoriesQuery {
    featured: Option<bool>,
}

#[derive(Deserialize)]
struct AdminCategoriesQuery {
    //query
    query: Option<String>,
    //sort zone
    sort_by: Option<String>, //Enum better? "id", "name", "image_id", "is_featured", "is_available"
    order: Option<String>,   //Enum better? "desc" / "asc"
    //filter zone
    only_featured: Option<bool>,
    only_available: Option<bool>,
    //pagination zone
    page: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
    page_size: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
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
