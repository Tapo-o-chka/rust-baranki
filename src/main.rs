mod entities;
mod middleware;
mod routes;

use axum::{http::StatusCode, response::Response, routing::get, Json};
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;
use std::sync::Arc;

use crate::entities::{primary_settup, setup_schema};
use crate::middleware::logging::{logging_middleware, to_response};
use crate::routes::api_router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("Databse url must be set");

    let db: DatabaseConnection = Database::connect(&database_url).await.unwrap();
    setup_schema(&db).await;

    let shared_db = Arc::new(db);

    primary_settup(shared_db.clone()).await;

    let mut app = api_router(shared_db);

    app = app
        .route("/", get(root))
        .layer(axum::middleware::from_fn(logging_middleware));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Running at {:?}", listener);
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Response {
    to_response(
        (
            StatusCode::OK,
            Json(json!({
                "message": "alive"
            })),
        ),
        Ok(()),
    )
}
