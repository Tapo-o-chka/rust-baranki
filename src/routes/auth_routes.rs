use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::Extension, http::StatusCode, response::IntoResponse, routing::post, Json, Router,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, Set};
use sea_orm::{QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::user::{self, Entity as UserEntity, Role};
use crate::middleware::auth::generate_token;

pub async fn auth_routes(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login))
        .layer(Extension(db))
}

// ROUTES
pub async fn register_user(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_user()` with payload: \n>{:?}",
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

    let password = match hash_password(&payload.password) {
        Ok(password) => password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "An internal server error occured"
                })),
            );
        }
    };

    let new_user = user::ActiveModel {
        username: Set(payload.username),
        password: Set(password),
        role: Set(Role::User),
        ..Default::default()
    };

    match user::Entity::insert(new_user).exec(&txn).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message": "User registered successfully"
            })),
        ),
        Err(err) => {
            println!("Error: {:?}", err);
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Username already exists"
                })),
            )
        }
    }
}

pub async fn login(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UserLogin>,
) -> impl IntoResponse {
    println!(
        "->> Called `login()` with payload: \n>{:?}",
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

    let result = UserEntity::find()
        .filter(user::Column::Username.eq(&*payload.username))
        .one(&txn)
        .await;

    match result {
        Ok(Some(model)) => match model.check_hash(&payload.password.clone()) {
            Ok(()) => match generate_token(model.id, model.role.to_string()).await {
                Ok(token) => (
                    StatusCode::OK,
                    Json(json!({
                        "token": token
                    })),
                ),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
            },
            Err(_) => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid username or password".to_string()
                })),
            ),
        },
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid username or password".to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "An internal server error occured".to_string()
            })),
        ),
    }
}

//utilities
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

//structs
#[derive(Deserialize, Clone, Debug)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserLogin {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct JWTSend {
    pub jwt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseMessage {
    message: String,
}
