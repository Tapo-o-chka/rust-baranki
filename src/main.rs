use axum::{routing::get, Router};
use sea_orm::{ConnectionTrait, Schema};
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::Mutex;

mod entities;
mod routes;
mod middleware;

use crate::entities::{
    cart::Entity as Crate,
    category::Entity as Category,
    user::Entity as User,
    product::Entity as Product,
};
use crate::routes::auth_routes::auth_routes;

pub async fn setup_schema(db: &DatabaseConnection) {
    let schema = Schema::new(db.get_database_backend());
    let create_cart_table = schema.create_table_from_entity(Crate);
    let create_category_table = schema.create_table_from_entity(Category);
    let create_user_table = schema.create_table_from_entity(User);
    let create_product_table = schema.create_table_from_entity(Product);
    
    db.execute(db.get_database_backend().build(&create_cart_table))
        .await
        .expect("Failed to create cart schema");
    db.execute(db.get_database_backend().build(&create_category_table))
        .await
        .expect("Failed to create category schema");
    db.execute(db.get_database_backend().build(&create_user_table))
        .await
        .expect("Failed to create user schema");
    db.execute(db.get_database_backend().build(&create_product_table))
        .await
        .expect("Failed to create product schema");
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("Databse url must be set");
    let db: DatabaseConnection = Database::connect(&database_url).await.unwrap();

    setup_schema(&db).await;

    let shared_db = Arc::new(Mutex::new(db));
    let user_routes = auth_routes(shared_db.clone()).await;

    let app = Router::new().route("/", get(root)).nest("/", user_routes);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Running at {:?}", listener);
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}
