use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

const SECRET: &str = "very_secret";

pub async fn jwt_middleware(mut req: Request<Body>, next: Next) -> impl IntoResponse {
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_header) = auth_header.to_str() {
            if auth_header.starts_with("Bearer ") {
                let token: &str = &auth_header[7..];
                match validate_token(token) {
                    Ok(claims) => {
                        req.extensions_mut().insert(claims.username.clone());
                        return next.run(req).await;
                    }
                    Err(_) => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(ResponseMessage {
                                message: "Invalid token".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
        }
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(ResponseMessage {
            message: "Missing or invalid Authorization header".to_string(),
        }),
    )
        .into_response()
}

fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseMessage {
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    username: String,
    exp: usize,
}
