use axum::{routing::get, Router};
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::Mutex;

mod entities;
mod routes;
mod middleware;

use crate::entities::setup_schema;
use crate::routes::{
    auth_routes::auth_routes, 
    category_routes::category_routes, 
    upload_routes::upload_routes
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("Databse url must be set");
    let db: DatabaseConnection = Database::connect(&database_url).await.unwrap();

    setup_schema(&db).await;

    let shared_db = Arc::new(Mutex::new(db));
    
    let user_routes = auth_routes(shared_db.clone()).await;
    let category_routes = category_routes(shared_db.clone()).await;
    let upload_routes = upload_routes(shared_db.clone()).await;

    let app = Router::new().route("/", get(root)).nest("/", user_routes).nest("/api", category_routes).nest("/api2", upload_routes);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Running at {:?}", listener);
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}
