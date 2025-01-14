use axum::{
    extract::Extension, http::StatusCode, response::IntoResponse, routing::get, Json, Router,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::entities::user::{ActiveModel, Entity as UserEntity, Role};
use crate::middleware::auth::{auth_middleware, Claims};

pub fn profile_routes() -> Router {
    Router::new()
        .route("/profile", get(get_profile).patch(patch_profile))
        .layer(axum::middleware::from_fn_with_state(
            Role::User,
            auth_middleware,
        ))
}

async fn get_profile(
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
        }
    };

    //yes, just a username.
    match UserEntity::find_by_id(user_id).one(&txn).await {
        Ok(Some(model)) => (
            StatusCode::OK,
            Json(json!({
                "username": format!("{}", model.username)
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Not found"
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

async fn patch_profile(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PatchProfile>,
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
        }
    };

    //yes, just a username.
    //yes, nested, i hate it.
    match UserEntity::find_by_id(user_id).one(&txn).await {
        Ok(Some(model)) => {
            let mut model: ActiveModel = model.into();
            model.username = Set(payload.username);
            let result = model.update(&txn).await.map(|_| ());
            match result {
                Ok(_) => match txn.commit().await {
                    Ok(_) => (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Resource patched successfully"
                        })),
                    ),
                    Err(_) => (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "This username is claimed"
                        })),
                    ),
                },
                Err(_) => {
                    let _ = txn.rollback().await;
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Failed to patch this resource"
                        })),
                    );
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Not found"
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error"
            })),
        ),
    }
}

#[derive(Deserialize)]
struct PatchProfile {
    username: String,
}
