use axum::{
    extract::Extension, http::StatusCode, middleware, response::Response, routing::get, Json,
    Router,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use validator::Validate;

use crate::entities::user::{ActiveModel, Entity as UserEntity, Role};
use crate::middleware::{
    auth::{auth_middleware, Claims},
    logging::{to_response, ApiError},
};
use crate::routes::auth_routes::USERNAME_REGEX;

pub fn profile_routes() -> Router {
    Router::new()
        .route("/profile", get(get_profile).patch(patch_profile))
        .layer(middleware::from_fn_with_state(Role::User, auth_middleware))
}

async fn get_profile(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
) -> Response {
    let user_id = claims.user_id;

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    //yes, just a username.
    match UserEntity::find_by_id(user_id).one(&txn).await {
        Ok(Some(model)) => to_response(
            (
                StatusCode::OK,
                Json(json!({
                    "username": format!("{}", model.username)
                })),
            ),
            Ok(()),
        ),
        Ok(None) => to_response(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized access"
                })),
            ),
            Err(ApiError::General("User profile not found".to_string())),
        ),
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn patch_profile(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PatchProfile>,
) -> Response {
    let user_id = claims.user_id;

    if let Some(err) = payload.validate().err() {
        return to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to validate username"
                })),
            ),
            Err(ApiError::ValidationFail(err.to_string())),
        );
    }

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
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
                    Ok(_) => to_response(
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource patched successfully"
                            })),
                        ),
                        Ok(()),
                    ),
                    Err(err) => to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "This username is claimed"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    ),
                },
                Err(err) => {
                    let _ = txn.rollback().await;
                    return to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to patch this resource"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    );
                }
            }
        }
        Ok(None) => to_response(
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Not found"
                })),
            ),
            Err(ApiError::General("User not found".to_string())),
        ),
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

#[derive(Deserialize, Validate)]
struct PatchProfile {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: String,
}
