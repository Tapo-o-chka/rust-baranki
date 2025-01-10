pub mod cart;

use axum::{middleware::from_fn_with_state, Router};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::entities::user::Role;
use crate::middleware::auth::{auth_middleware, AuthState};
use cart::cart_router;

pub fn user_api_router(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .nest("/", cart_router(db.clone()))
        .layer(from_fn_with_state(
            AuthState {
                db: db.clone(),
                role: Role::User,
            },
            auth_middleware,
        ))
}
