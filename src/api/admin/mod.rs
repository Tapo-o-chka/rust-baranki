pub mod category;
pub mod product;
pub mod upload;

use axum::{middleware::from_fn_with_state, Router};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use category::admin_category_router;
use product::admin_product_router;
use upload::upload_router;

use crate::entities::user::Role;
use crate::middleware::auth::{auth_middleware, AuthState};

pub fn admin_api_router(db: Arc<DatabaseConnection>) -> Router {
    let admin_category_router = admin_category_router(db.clone());
    let admin_product_router = admin_product_router(db.clone());
    let upload_router = upload_router(db.clone());

    Router::new()
        .nest("/", admin_category_router)
        .nest("/", admin_product_router)
        .nest("/", upload_router)
        .layer(from_fn_with_state(
            AuthState {
                db: db.clone(),
                role: Role::Admin,
            },
            auth_middleware,
        ))
}
