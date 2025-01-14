use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::user::{self, Entity as UserEntity, Role};
use crate::middleware::auth::{auth_middleware, generate_token};

pub fn auth_routes() -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login))
}

pub fn admin_users_routes() -> Router {
    Router::new()
        .route("/user", get(get_users).post(create_user))
        .route("/user/:id", delete(admin_delete_user).patch(patch_user))
        .layer(axum::middleware::from_fn_with_state(
            Role::Admin,
            auth_middleware,
        ))
}

// ROUTES
async fn register_user(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    println!(
        "->> Called `register_user()` with payload: \n>{:?}",
        payload.clone()
    );

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

    let password = match hash_password(&payload.password) {
        Ok(password) => password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "An internal server error occured"
                })),
            );
        }
    };

    let new_user = user::ActiveModel {
        username: Set(payload.username),
        password: Set(password),
        role: Set(Role::User),
        ..Default::default()
    };

    match user::Entity::insert(new_user).exec(&txn).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message": "User registered successfully"
            })),
        ),
        Err(err) => {
            println!("Error: {:?}", err);
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Username already exists"
                })),
            )
        }
    }
}

async fn login(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UserLogin>,
) -> impl IntoResponse {
    println!(
        "->> Called `login()` with payload: \n>{:?}",
        payload.clone()
    );

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

    let result = UserEntity::find()
        .filter(user::Column::Username.eq(&*payload.username))
        .one(&txn)
        .await;

    match result {
        Ok(Some(model)) => match model.check_hash(&payload.password.clone()) {
            Ok(()) => match generate_token(model.id, model.role.to_string()).await {
                Ok(token) => (
                    StatusCode::OK,
                    Json(json!({
                        "token": token
                    })),
                ),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
            },
            Err(_) => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid username or password".to_string()
                })),
            ),
        },
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid username or password".to_string()
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "An internal server error occured".to_string()
            })),
        ),
    }
}

async fn create_user(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<AdminCreateUser>,
) -> impl IntoResponse {
    println!(
        "->> Called `create_user()` with payload: \n>{:?}",
        payload.clone()
    );

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

    let password = match hash_password(&payload.password) {
        Ok(password) => password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "An internal server error occured"
                })),
            );
        }
    };

    let new_user = user::ActiveModel {
        username: Set(payload.username),
        password: Set(password),
        role: Set(payload.role),
        ..Default::default()
    };

    match user::Entity::insert(new_user).exec(&txn).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message": "User registered successfully"
            })),
        ),
        Err(err) => {
            println!("Error: {:?}", err);
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Username already exists"
                })),
            )
        }
    }
}

async fn get_users(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(query): Query<UsersQuery>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    };

    let order = match query.order.as_deref() {
        Some("desc") => sea_orm::Order::Desc,
        _ => sea_orm::Order::Asc,
    };

    let sort_users = match query.sort_by.as_deref() {
        Some("username") => user::Column::Username,
        Some("role") => user::Column::Role,
        _ => user::Column::Id,
    };

    let mut user_finder = user::Entity::find();

    if let Some(role) = query.role {
        user_finder = user_finder.filter(user::Column::Role.eq(role));
    }

    //Well, simple enough.
    if let Some(query) = query.query {
        let mut query_condition =
            Condition::any().add(user::Column::Username.contains(query.clone()));
        let id_search = query.parse::<u32>().ok();
        if let Some(id) = id_search {
            query_condition = query_condition.add(user::Column::Id.eq(id));
        }

        user_finder = user_finder.filter(query_condition);
    }

    let users: Vec<AdminUserResponse> = match user_finder
        .order_by(sort_users, order)
        .select_only() //to select specific columns
        .column_as(user::Column::Id, "id")
        .column_as(user::Column::Role, "role")
        .column_as(user::Column::Username, "username")
        .into_model::<AdminUserResponse>()
        .all(&txn)
        .await
    {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    };

    Json(users).into_response()
}

async fn admin_delete_user(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

    match UserEntity::find_by_id(id).one(&txn).await {
        Ok(Some(entry)) => {
            let entry: user::ActiveModel = entry.into();
            let result = entry.delete(&txn).await;
            match result {
                Ok(_) => {
                    let _ = txn.commit().await;
                    (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Resource deleted successfully"
                        })),
                    )
                }
                Err(_) => {
                    let _ = txn.rollback().await;
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Failed to delete this resource"
                        })),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("No related entry with {} id was found.", id)
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error."
            })),
        ),
    }
}

async fn patch_user(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchUser>, //Safety ahhhh
) -> impl IntoResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error"
                })),
            );
        }
    };

    match UserEntity::find_by_id(id).one(&txn).await {
        Ok(Some(user)) => {
            let mut user: user::ActiveModel = user.into();

            if let Some(username) = payload.username {
                if username != "" {
                    user.username = Set(username);
                }
            }

            if let Some(password) = payload.password {
                if password != "" {
                    let password = match hash_password(&password) {
                        Ok(password) => password,
                        Err(_) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "An internal server error occured"
                                })),
                            );
                        }
                    };
                    user.password = Set(password);
                }
            }

            if let Some(role) = payload.role {
                user.role = Set(role);
            }

            let result: Result<(), DbErr> = user.update(&txn).await.map(|_| ());

            match result {
                Ok(_) => match txn.commit().await {
                    Ok(_) => (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Resource patched successfully"
                        })),
                    ),
                    Err(_) => (
                        StatusCode::CONFLICT,
                        Json(json!({
                            "error": "Username unique constraint failed"
                        }))
                    )
                }
                Err(_) => {
                    let _ = txn.rollback().await;
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Failed to patch this resource"
                        })),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("No related entry with {} id was found.", id)
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error."
            })),
        ),
    }
}

//utilities
fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

//structs
#[derive(Deserialize, Clone, Debug)]
struct CreateUser {
    username: String,
    password: String,
}

#[derive(Deserialize, Clone, Debug)]
struct AdminCreateUser {
    username: String,
    password: String,
    role: Role,
}

#[derive(Debug, Deserialize, Clone)]
struct UserLogin {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseMessage {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PatchUser {
    role: Option<Role>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize, Serialize, FromQueryResult)]
struct AdminUserResponse {
    id: i32,
    username: String,
    role: Role,
}

#[derive(Deserialize)]
struct UsersQuery {
    //Query
    query: Option<String>,
    //Sort zone
    sort_by: Option<String>, //Enum better?? "id", "username", "role"
    order: Option<String>,   //Enum better??
    //filter zone
    role: Option<Role>, //incoming should be None, "user" or "admin"
}
