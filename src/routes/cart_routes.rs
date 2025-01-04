use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    middleware::{self},
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set, TransactionTrait
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::entities::{cart, product};
use crate::middleware::auth::jwt_middleware;

//ROUTERS
pub async fn cart_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route("/cart", get(get_cart).post(add_product))
        .route("/cart/:id", patch(patch_entry).delete(remove_product))
        .layer(middleware::from_fn(jwt_middleware))
        .layer(Extension(db))
}

async fn get_cart(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Extension(userd_id): Extension<i32>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => match cart::Entity::find()
            .filter(cart::Column::UserId.eq(userd_id))
            .into_json()
            .all(&txn)
            .await
        {
            Ok(entries) => (StatusCode::OK, Json(entries)).into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            )
                .into_response(),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

async fn add_product(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Extension(userd_id): Extension<i32>,
    Json(payload): Json<AddProduct>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => match product::Entity::find_by_id(payload.product_id)
            .one(&txn)
            .await
        {
            Ok(Some(_)) => {
                if payload.quantity > 0 {
                    let new_entry = cart::ActiveModel {
                        user_id: Set(userd_id),
                        product_id: Set(payload.product_id),
                        quantity: Set(payload.quantity),
                        ..Default::default()
                    };
                    match cart::Entity::insert(new_entry).exec(&txn).await {
                        Ok(_) => {
                            let _ = txn.commit().await;
                            (
                                StatusCode::CREATED,
                                Json(json!({
                                    "message": "Added successfully"
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
                } else {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Quantity should be greater than 0"
                        })),
                    )
                }
            }
            Ok(None) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("No product with {} id was found", payload.product_id)
                })),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

async fn remove_product(
    Path(id): Path<i32>,
    Extension(userd_id): Extension<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => match cart::Entity::find_by_id(id)
            .filter(cart::Column::UserId.eq(userd_id))
            .one(&txn)
            .await
        {
            Ok(Some(entry)) => {
                let entry: cart::ActiveModel = entry.into();
                let result = entry.delete(&txn).await;
                match result {
                    Ok(_) => {
                        let _ = txn.commit().await;
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource deleted successfully"
                            })),
                        )
                    }
                    Err(_) => {
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
                    "error": format!("No related entry with {} id was found.", id)
                })),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

async fn patch_entry(
    Path(id): Path<i32>,
    Extension(userd_id): Extension<i32>,
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<PatchCart>,
) -> impl IntoResponse {
    let db = db.lock().await;
    match db.begin().await {
        Ok(txn) => match cart::Entity::find_by_id(id)
            .filter(cart::Column::UserId.eq(userd_id))
            .one(&txn)
            .await
        {
            Ok(Some(entry)) => {
                let mut entry: cart::ActiveModel = entry.into();
                
                let result: Result<(), DbErr> = match payload.quantity {
                    value if value == 0 => {
                        entry.delete(&txn).await.map(|_| ())
                    }
                    _ => {
                        entry.quantity = Set(payload.quantity);
                        entry.update(&txn).await.map(|_| ())
                    }
                };
                match result {
                    Ok(_) => {
                        let _ = txn.commit().await;
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource patched successfully"
                            })),
                        )
                    }
                    Err(_) => {
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
                    "error": format!("No related entry with {} id was found.", id)
                })),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

//Structs
#[derive(Deserialize)]
struct AddProduct {
    product_id: i32,
    quantity: u32, //maybe u16 is enough...
}

#[derive(Deserialize)]
struct PatchCart {
    quantity: u32
}
