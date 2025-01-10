use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::entities::{cart, cart::Entity as CartEntity, product};
use crate::middleware::auth::Claims;

//ROUTERS
pub fn cart_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/cart", get(get_cart).post(add_product))
        .route("/cart/:id", patch(patch_entry).delete(remove_product))
        .layer(Extension(db))
}

async fn get_cart(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = claims.user_id;
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
    match CartEntity::find()
        .filter(cart::Column::UserId.eq(user_id))
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
    }
}

async fn add_product(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AddProduct>,
) -> impl IntoResponse {
    //too nested
    println!("->> Called `add_product` with payload: {:?}", payload);
    let user_id = claims.user_id;
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
    match product::Entity::find_by_id(payload.product_id)
        .one(&txn)
        .await
    {
        Ok(Some(_)) => {
            if payload.quantity > 0 {
                if let Ok(Some(entry)) = CartEntity::find()
                    .filter(cart::Column::ProductId.eq(payload.product_id))
                    .filter(cart::Column::UserId.eq(user_id))
                    .one(&txn)
                    .await
                {
                    let mut entry: cart::ActiveModel = entry.into();
                    entry.quantity = Set(entry.quantity.unwrap() + payload.quantity);
                    let result = entry.update(&txn).await.map(|_| ());
                    match result {
                        Ok(_) => {
                            let _ = txn.commit().await;
                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "message": "Resource patched successfully"
                                })),
                            );
                        }
                        Err(_) => {
                            let _ = txn.rollback().await;
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "error": "Failed to patch this resource"
                                })),
                            );
                        }
                    };
                };
                let new_entry = cart::ActiveModel {
                    user_id: Set(user_id),
                    product_id: Set(payload.product_id),
                    quantity: Set(payload.quantity),
                    ..Default::default()
                };
                match CartEntity::insert(new_entry).exec(&txn).await {
                    Ok(_) => match txn.commit().await {
                        Ok(_) => (
                            StatusCode::CREATED,
                            Json(json!({
                                "message": "Added successfully"
                            })),
                        ),
                        Err(_) => {
                            println!("Failed to commit");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Internal server error"
                                })),
                            )
                        }
                    },
                    Err(_) => {
                        println!("Internal server error on adding cart entry");
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
                println!("Error: quanity should be greater thatn 0");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Quantity should be greater than 0"
                    })),
                )
            }
        }
        Ok(None) => {
            println!("Error: no product found");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("No product with {} id was found", payload.product_id)
                })),
            )
        }
        Err(_) => {
            println!("Db search failed??");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            )
        }
    }
}

async fn remove_product(
    Path(id): Path<i32>,
    Extension(claims): Extension<Claims>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    let user_id = claims.user_id;
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

    match CartEntity::find_by_id(id)
        .filter(cart::Column::UserId.eq(user_id))
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
    }
}

async fn patch_entry(
    Path(id): Path<i32>,
    Extension(claims): Extension<Claims>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchCart>,
) -> impl IntoResponse {
    let user_id = claims.user_id;
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

    match CartEntity::find_by_id(id)
        .filter(cart::Column::UserId.eq(user_id))
        .one(&txn)
        .await
    {
        Ok(Some(entry)) => {
            let mut entry: cart::ActiveModel = entry.into();

            let result: Result<(), DbErr> = match payload.quantity {
                value if value == 0 => entry.delete(&txn).await.map(|_| ()),
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
    }
}

//Structs
#[derive(Deserialize, Debug)]
struct AddProduct {
    product_id: i32,
    quantity: u32, //maybe u16 is enough...
}

#[derive(Deserialize)]
struct PatchCart {
    quantity: u32,
}
