use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tracing::{error, info, warn};

pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status();
    let elapsed = start.elapsed();
    println!(
        "\nCalled: {} '{}'\n> Status: {}\n> Time: {:#?}",
        method, uri, status, elapsed
    );
    if let Some(data) = response.extensions().get::<Result<(), ApiError>>() {
        println!("> Logged Data: {:?}", data);
        match data {
            Ok(_) => info!(
                method = %method,
                uri = %uri,
                status = %status,
                elapsed = ?elapsed,
                "Processed request"
            ),
            Err(value) => error!(
                method = %method,
                uri = %uri,
                status = %status,
                elapsed = ?elapsed,
                value = %value.to_string(),
                "Failed to process request"
            ),
        }
    }
    warn!(
        method = %method,
        uri = %uri,
        status = %status,
        elapsed = ?elapsed,
        "Processed request, but no Response extension is set"
    );

    response
}

#[derive(Clone, Debug)]
pub enum ApiError {
    TransactionCreationFailed,
    PasswordHashFailed(String),
    General(String),
    TokenGenerationFailed(String),
    DbError(String),
    ValidationFail(String),
}

impl ToString for ApiError {
    fn to_string(&self) -> String {
        match self {
            ApiError::TransactionCreationFailed => "Failed to create transaction".into(),
            ApiError::PasswordHashFailed(value) => format!("Failed to hash password {value}"),
            ApiError::General(value) => value.clone(),
            ApiError::TokenGenerationFailed(value) => format!("Failed to generate token: {value}"),
            ApiError::DbError(value) => format!("Database error: {value}"),
            ApiError::ValidationFail(value) => format!("Failed to validate: {value}"),
        }
    }
}

pub fn to_response<T: IntoResponse>(
    response: T,               //The response that we are sending + StatusCode
    ext: Result<(), ApiError>, //The extension, that we want to give logging middleware
) -> Response {
    let mut response = response.into_response();

    response.extensions_mut().insert(ext);

    response
}
