pub mod auth_routes;
pub mod cart_routes;
pub mod category_routes;
pub mod product_routes;
pub mod profile_routes;
pub mod upload_routes;

use axum::{Router, Extension};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use {
    auth_routes::{auth_routes, admin_users_routes},
    cart_routes::{cart_routes, admin_cart_routes},
    profile_routes::profile_routes,
    category_routes::{admin_category_routes, category_routes},
    product_routes::{admin_product_routes, product_routes},
    upload_routes::{public_image_router, upload_routes},
};

pub fn api_router(db: Arc<DatabaseConnection>) -> Router {
    //does it need to be async?
    let user_routes = auth_routes();
    let category_routes = category_routes();
    let admin_category_routes = admin_category_routes();
    let product_routes = product_routes();
    let admin_product_routes = admin_product_routes();
    let upload_routes = upload_routes();
    let cart_routes = cart_routes();
    let public_image_router = public_image_router();
    let profile_router = profile_routes();
    let admin_cart_routes = admin_cart_routes();
    let admin_users_router = admin_users_routes();

    Router::new()
        .nest("/", user_routes)
        .nest("/", public_image_router)
        .nest("/api", category_routes)
        .nest("/api", product_routes)
        .nest("/api", upload_routes)
        .nest("/api", cart_routes)
        .nest("/api", profile_router)
        .nest("/api/admin", admin_category_routes)
        .nest("/api/admin", admin_product_routes)
        .nest("/api/admin", admin_cart_routes)
        .nest("/api/admin", admin_users_router)
        .layer(Extension(db))
}
