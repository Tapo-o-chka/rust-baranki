use crate::entities::user::{self, Entity as UserEntity, Role};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use dotenvy::dotenv;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    println!("Loaded middleware.");
    let db = state.db;
    let role = state.role;

    println!("Loaded states.");
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    println!("Got to token");
    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => match header.strip_prefix("Bearer ") {
            Some(token) => token,
            _ => {
                println!("Token failed: \n{:?}", header);
                return Err(StatusCode::UNAUTHORIZED);
            }
        },
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    println!("Got token");
    let claims: Claims = match validate_token(db.clone(), token, role).await {
        Ok(claims) => claims,
        Err(errr) => {
            println!("Unun {:?}", errr);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    println!("Set claims: {:?}", claims);
    req.extensions_mut().insert(claims);
    println!("Gone to next");
    Ok(next.run(req).await)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub role: String,
    pub exp: usize,
}

#[derive(Clone, Debug)]
pub struct AuthState {
    pub db: Arc<DatabaseConnection>,
    pub role: Role,
}

pub async fn generate_token(user_id: i32, role: String) -> Result<String, AuthMiddlewareError> {
    let exp = match Utc::now()
        .checked_add_signed(Duration::hours(24))
        .ok_or(AuthMiddlewareError::GenerationFail)
    {
        Ok(data) => data.timestamp() as usize,
        Err(_) => {
            return Err(AuthMiddlewareError::GenerationFail);
        }
    };

    let claims = Claims { user_id, role, exp };

    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_secret_key().as_bytes()),
    ) {
        Ok(token) => Ok(token),
        Err(_) => Err(AuthMiddlewareError::GenerationFail),
    }
}

pub async fn validate_token(
    db: Arc<DatabaseConnection>,
    token: &str,
    req_role: Role,
) -> Result<Claims, AuthMiddlewareError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_secret_key().as_bytes()),
        &validation,
    ) {
        Ok(data) => data,
        Err(_) => {
            return Err(AuthMiddlewareError::TokenExpired);
        }
    };

    let claims = token_data.claims;

    if let Ok(role) = Role::from_str(&claims.role) {
        match UserEntity::find_by_id(claims.user_id)
            .filter(user::Column::Role.eq(role))
            .one(&*db)
            .await
        {
            Ok(Some(_)) => {
                if role == req_role {
                    return Ok(claims);
                } else {
                    return Err(AuthMiddlewareError::InvalidUserOrRole);
                }
            }
            Ok(None) => {
                return Err(AuthMiddlewareError::InvalidUserOrRole);
            }
            Err(_) => {
                return Err(AuthMiddlewareError::InternalServerError);
            }
        }
    }

    Err(AuthMiddlewareError::ValidationFail)
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthMiddlewareError {
    #[error("Invalid user id or role")]
    InvalidUserOrRole,
    #[error("Token expired")]
    TokenExpired,
    #[error("Failed to validate token")]
    ValidationFail,
    #[error("Failed to generate token")]
    GenerationFail,
    #[error("Internal server error")]
    InternalServerError,
}

fn get_secret_key() -> String {
    dotenv().ok();
    std::env::var("SECRET").expect("SECRET not found in .env file")
}
