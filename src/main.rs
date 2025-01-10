use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;

mod entities;
mod middleware;
mod api;

use crate::entities::{setup_schema, primary_settup};
use crate::api::create_api_router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL").expect("Databse url must be set");
    let db: DatabaseConnection = Database::connect(&database_url).await.unwrap();
    setup_schema(&db).await;

    let shared_db = Arc::new(db);

    primary_settup(shared_db.clone()).await;
    
    let app = create_api_router(shared_db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Running at {:?}", listener);
    axum::serve(listener, app).await.unwrap();
}
