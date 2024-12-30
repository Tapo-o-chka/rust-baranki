use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::Extension,
    http::StatusCode,
    middleware::{self},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::entities::{user, user::Entity as UserEntity};
use crate::middleware::auth::jwt_middleware;

const SECRET: &str = "very_secret";

pub async fn auth_routes(db: Arc<Mutex<DatabaseConnection>>) -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login))
        .route(
            "/protected",
            get(protected).layer(middleware::from_fn(jwt_middleware)),
        )
        .layer(Extension(db))
}

// ROUTES
pub async fn register_user(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_user()` with payload: \n>{:?}",
        payload.clone()
    );
    let db = db.lock().await;

    match hash_password(&payload.password) {
        Ok(password) => {
            let new_user = user::ActiveModel {
                username: Set(payload.username.clone()),
                password: Set(password.clone()),
                ..Default::default()
            };
            match user::Entity::insert(new_user).exec(&*db).await {
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
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "An internal server error occured"
            })),
        ),
    }
}

pub async fn login(
    Extension(db): Extension<Arc<Mutex<DatabaseConnection>>>,
    Json(payload): Json<UserLogin>,
) -> impl IntoResponse {
    println!(
        "->> Called `login()` with payload: \n>{:?}",
        payload.clone()
    );
    let db = db.lock().await;
    let result = UserEntity::find()
        .filter(user::Column::Username.eq(&*payload.username))
        .one(&*db)
        .await;

    match result {
        Ok(Some(model)) => match model.check_hash(&payload.password.clone()) {
            Ok(()) => {
                let token = generate_token(&*payload.username);
                (
                    StatusCode::OK,
                    Json(json!({
                        "token": token
                    })),
                )
            }
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

async fn protected(username: Extension<String>) -> impl IntoResponse {
    //println!("{}", format!("Welcome, {:?}! This is a protected route.", username.parse::<String>().expect("Cant unwrap username")));
    
    Json(ResponseMessage {
        message: format!("Welcome, {}! This is a protected route.", username.parse::<String>().expect("Cant unwrap username")),
    })
    .into_response()
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

fn generate_token(username: &str) -> String {
    let claims = Claims {
        username: username.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::days(1)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_ref()),
    )
    .unwrap_or(format!(
        ">DEBUG PRINT FOR `fn generate token:\n> Claims: {:?}\n> Secret: {:?}",
        &claims, &SECRET
    ))
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
struct Claims {
    username: String,
    exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseMessage {
    message: String,
}