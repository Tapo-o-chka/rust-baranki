use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, FromQueryResult,
    JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::{
    category, image,
    product::{self, Entity as ProductEntity},
    user,
    user::Role,
};
use crate::middleware::auth::auth_middleware;

//ROUTERS
pub fn product_routes() -> Router {
    Router::new()
        .route("/product", get(get_products))
        .route("/product/:id", get(get_product))
}

pub fn admin_product_routes() -> Router {
    Router::new()
        .route("/product", post(create_product).get(admin_get_products))
        .route("/product/:id", patch(patch_product).delete(delete_product))
        .layer(axum::middleware::from_fn_with_state(
            Role::Admin,
            auth_middleware,
        ))
}

//ROUTES
async fn create_product(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateProduct>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_product()` with payload: \n>{:?}",
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

    match user::Entity::find_by_id(payload.image_id).one(&txn).await {
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
                Ok(_) => match txn.commit().await {
                    Ok(_) => (
                        StatusCode::CREATED,
                        Json(json!({
                            "message": "Product created successfully"
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
    }
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

    let mut condition = Condition::all();

    //Filter zone
    if let Some(price_bottom) = params.price_bottom {
        condition = condition.add(product::Column::Price.gte(price_bottom));
    }
    if let Some(price_top) = params.price_top {
        condition = condition.add(product::Column::Price.lte(price_top));
    }
    if let Some(category_ids) = params.category_ids {
        condition = condition.add(product::Column::CategoryId.is_in(category_ids));
    }
    if params.only_available.unwrap_or(false) {
        condition = condition.add(product::Column::IsAvailable.eq(true));
    }

    //Sorting zone
    let order = match params.order.as_deref() {
        Some("desc") => sea_orm::Order::Desc,
        _ => sea_orm::Order::Asc,
    };

    let sort_column = match params.sort_by.as_deref() {
        Some("price") => product::Column::Price,
        Some("is_available") => product::Column::IsAvailable,
        _ => product::Column::Name,
    };

    condition = condition.add(category::Column::IsAvailable.eq(true));

    //Pagination zone
    let page: u64 = params.page.unwrap_or(1);
    let page_size: u64 = params.page_size.unwrap_or(10);

    //Building response
    let mut items = product::Entity::find();

    //adding query
    if let Some(query) = params.query {
        let mut query_condition = Condition::any().add(category::Column::Name.contains(query.clone()));
        let id_search = query.parse::<u32>().ok();
        if let Some(id) = id_search {
            query_condition = query_condition.add(category::Column::Id.eq(id));
        }

        items = items.filter(query_condition);
    }

    let items = items
        .filter(condition)
        .join(JoinType::InnerJoin, product::Relation::Category.def())
        .column_as(product::Column::Id, "product_id")
        .column_as(product::Column::Name, "name")
        .column_as(product::Column::Price, "price")
        .column_as(product::Column::Description, "description")
        .column_as(product::Column::ImageId, "image_id")
        .column_as(category::Column::Name, "category_name")
        .order_by(sort_column, order)
        .limit(page_size)
        .offset((page - 1) * page_size)
        .into_model::<ProductResponse>()
        .all(&txn)
        .await
        .unwrap_or_else(|_| vec![]);

    Json(items).into_response()
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
        .join(JoinType::InnerJoin, product::Relation::Category.def())
        .column_as(product::Column::Id, "id")
        .column_as(product::Column::Name, "name")
        .column_as(product::Column::Price, "price")
        .column_as(product::Column::Description, "description")
        .column_as(product::Column::ImageId, "image_id")
        .column_as(category::Column::Name, "category_name")
        .into_model::<ProductResponse>()
        .one(&txn)
        .await;

    match result {
        Ok(Some(prod)) => (StatusCode::OK, Json(prod)).into_response(),
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

async fn admin_get_products(
    Query(params): Query<AdminProductsQuery>,
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
    if let Some(price_bottom) = params.price_bottom {
        condition = condition.add(product::Column::Price.gte(price_bottom));
    }
    if let Some(price_top) = params.price_top {
        condition = condition.add(product::Column::Price.lte(price_top));
    }
    if let Some(category_ids) = params.category_ids {
        condition = condition.add(product::Column::CategoryId.is_in(category_ids));
    }
    if params.only_available.unwrap_or(false) {
        condition = condition.add(product::Column::IsAvailable.eq(true));
    }
    if params.only_featured.unwrap_or(false) {
        condition = condition.add(product::Column::IsFeatured.eq(true))
    }

    //Sorting zone
    let order = match params.order.as_deref() {
        Some("desc") => sea_orm::Order::Desc,
        _ => sea_orm::Order::Asc,
    };

    let sort_column = match params.sort_by.as_deref() {
        Some("price") => product::Column::Price,
        Some("is_available") => product::Column::IsAvailable,
        Some("is_featured") => product::Column::IsFeatured,
        Some("name") => product::Column::Name,
        Some("image_id") => product::Column::ImageId,
        Some("category_id") => product::Column::CategoryId,
        _ => product::Column::Id,
    };

    condition = condition.add(category::Column::IsAvailable.eq(true));

    //Pagination zone
    let page: u64 = params.page.unwrap_or(1);
    let page_size: u64 = params.page_size.unwrap_or(10);

    //Response buidling
    let mut items = product::Entity::find();

    //adding query
    if let Some(query) = params.query {
        let mut query_condition = Condition::any().add(category::Column::Name.contains(query.clone()));
        let id_search = query.parse::<u32>().ok();
        if let Some(id) = id_search {
            query_condition = query_condition.add(category::Column::Id.eq(id));
        }

        items = items.filter(query_condition);
    }

    let items = items
        .filter(condition)
        .order_by(sort_column, order)
        .limit(page_size)
        .offset((page - 1) * page_size)
        .all(&txn)
        .await
        .unwrap_or_else(|_| vec![]);

    Json(items).into_response()
}

async fn patch_product(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchProductPayload>,
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
                        );
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
                        );
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

async fn delete_product(
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
    let result = ProductEntity::find_by_id(id).one(&txn).await;
    match result {
        Ok(Some(product)) => {
            let product: product::ActiveModel = product.into();
            let result = product.delete(&txn).await;
            match result {
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
    //query
    query: Option<String>,
    //sort zone
    sort_by: Option<String>, //Enum better?? "price", "is_available", "name"
    order: Option<String>,   //Enum better??
    //filter zone
    price_top: Option<i32>,
    price_bottom: Option<i32>,
    category_ids: Option<Vec<i32>>,
    only_available: Option<bool>,
    //pagination zone
    page: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
    page_size: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
}

#[derive(Deserialize)]
struct AdminProductsQuery {
    //query
    query: Option<String>,
    //sort zone
    sort_by: Option<String>, //Enum better?? "id,", "price", "is_available", "is_featured", "name", "image_id", "category_id"
    order: Option<String>,   //Enum better??
    //filter zone
    price_top: Option<i32>,
    price_bottom: Option<i32>,
    category_ids: Option<Vec<i32>>,
    only_available: Option<bool>,
    only_featured: Option<bool>,
    //pagination zone
    page: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
    page_size: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
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

#[derive(Serialize, FromQueryResult)]
struct ProductResponse {
    id: i32,
    name: String,
    price: f32,
    description: String,
    image_id: i32,
    category_name: String,
}
