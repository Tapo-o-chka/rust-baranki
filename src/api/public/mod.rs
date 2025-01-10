pub mod auth;
pub mod category;
pub mod product;
pub mod uploads;

use axum::Router;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use auth::auth_router;
use category::category_router;
use product::product_router;
use uploads::uploads_router;

pub fn public_api_router(db: Arc<DatabaseConnection>) -> Router {
    let auth_router = auth_router(db.clone());
    let category_router = category_router(db.clone());
    let product_router = product_router(db.clone());
    let uploads_router = uploads_router(db.clone());

    Router::new()
        .nest("/", auth_router)
        .nest("/", category_router)
        .nest("/", product_router)
        .nest("/", uploads_router)
}
