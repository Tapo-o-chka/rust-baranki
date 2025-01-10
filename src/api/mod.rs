pub mod admin;
pub mod public;
pub mod user;

use axum::Router;
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use public::public_api_router;
use user::user_api_router;
use admin::admin_api_router;

pub fn create_api_router(shared_db: Arc<DatabaseConnection>) -> Router {

    Router::new()
        .nest("/api", public_api_router(shared_db.clone()))
        .nest("/api", user_api_router(shared_db.clone()))
        .nest("/api/admin", admin_api_router(shared_db.clone()))
}