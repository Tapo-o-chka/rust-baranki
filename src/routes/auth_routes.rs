use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    middleware,
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use validator::Validate;

use crate::entities::user::{self, Entity as UserEntity, Role};
use crate::middleware::{
    auth::{auth_middleware, generate_token},
    logging::{to_response, ApiError},
};

pub fn auth_routes() -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login))
}

pub fn admin_users_routes() -> Router {
    //Yes, this looks like routes for ./profile_routes.rs
    Router::new()
        .route("/user", get(get_users).post(create_user))
        .route("/user/:id", delete(admin_delete_user).patch(patch_user))
        .layer(middleware::from_fn_with_state(Role::Admin, auth_middleware))
}

// ROUTES
async fn register_user(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<RegisterUser>,
) -> Response {
    if let Some(err) = payload.validate().err() {
        return to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to validate username or password"
                })),
            ),
            Err(ApiError::ValidationFail(err.to_string())),
        );
    }

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            )
        }
    };

    let password = match hash_password(&payload.password) {
        Ok(password) => password,
        Err(err) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "An internal server error occured"
                    })),
                ),
                Err(ApiError::PasswordHashFailed(err.to_string())),
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
        Ok(_) => to_response(
            (
                StatusCode::CREATED,
                Json(json!({
                    "message": "User registered successfully"
                })),
            ),
            Ok(()),
        ),
        Err(err) => to_response(
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Username already exists"
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn login(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UserLogin>,
) -> Response {
    if let Some(err) = payload.validate().err() {
        return to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "name should be at least 3 characters long"
                })),
            ),
            Err(ApiError::ValidationFail(err.to_string())),
        );
    }

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
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
                Ok(token) => to_response(
                    (
                        StatusCode::OK,
                        Json(json!({
                            "token": token
                        })),
                    ),
                    Ok(()),
                ),
                Err(err) => to_response(
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Internal server error"
                        })),
                    ),
                    Err(ApiError::TokenGenerationFailed(err.to_string())),
                ),
            },
            Err(err) => to_response(
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Invalid username or password".to_string()
                    })),
                ),
                Err(ApiError::General(err)),
            ),
        },
        Ok(None) => to_response(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid username or password".to_string()
                })),
            ),
            Err(ApiError::General(
                "Invalid username or password".to_string(),
            )),
        ),
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "An internal server error occured".to_string()
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn create_user(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateUser>,
) -> Response {
    if let Some(err) = payload.validate().err() {
        return to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to validate username or password"
                })),
            ),
            Err(ApiError::ValidationFail(err.to_string())),
        );
    }

    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    let password = match hash_password(&payload.password) {
        Ok(password) => password,
        Err(err) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "An internal server error occured"
                    })),
                ),
                Err(ApiError::PasswordHashFailed(err.to_string())),
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
        Ok(_) => to_response(
            (
                StatusCode::CREATED,
                Json(json!({
                    "message": "User registered successfully"
                })),
            ),
            Ok(()),
        ),
        Err(err) => to_response(
            (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "Username already exists"
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn get_users(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(query): Query<UsersQuery>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
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
        Err(err) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::DbError(err.to_string())),
            );
        }
    };

    to_response(Json(users), Ok(()))
}

async fn admin_delete_user(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    match UserEntity::find_by_id(id).one(&txn).await {
        Ok(Some(entry)) => {
            let entry: user::ActiveModel = entry.into();
            let result = entry.delete(&txn).await;
            match result {
                Ok(_) => match txn.commit().await {
                    Ok(_) => to_response(
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource deleted successfully"
                            })),
                        ),
                        Ok(()),
                    ),
                    Err(err) => to_response(
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Internal server error"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    ),
                },
                Err(err) => {
                    let _ = txn.rollback().await;
                    to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to delete this resource"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    )
                }
            }
        }
        Ok(None) => {
            let tmp = format!("No related entry with {} id was found.", id);
            to_response(
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": tmp
                    })),
                ),
                Err(ApiError::General(tmp)),
            )
        }
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
        ),
    }
}

async fn patch_user(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchUser>,
) -> Response {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return to_response(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Internal server error"
                    })),
                ),
                Err(ApiError::TransactionCreationFailed),
            );
        }
    };

    match UserEntity::find_by_id(id).one(&txn).await {
        Ok(Some(user)) => {
            let mut user: user::ActiveModel = user.into();

            if let Some(username) = payload.username {
                user.username = Set(username);
            }

            if let Some(password) = payload.password {
                let password = match hash_password(&password) {
                    Ok(password) => password,
                    Err(err) => {
                        return to_response(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "An internal server error occured"
                                })),
                            ),
                            Err(ApiError::PasswordHashFailed(err.to_string())),
                        );
                    }
                };
                user.password = Set(password);
            }

            if let Some(role) = payload.role {
                user.role = Set(role);
            }

            let result: Result<(), DbErr> = user.update(&txn).await.map(|_| ());

            match result {
                Ok(_) => match txn.commit().await {
                    Ok(_) => to_response(
                        (
                            StatusCode::OK,
                            Json(json!({
                                "message": "Resource patched successfully"
                            })),
                        ),
                        Ok(()),
                    ),
                    Err(err) => to_response(
                        (
                            StatusCode::CONFLICT,
                            Json(json!({
                                "error": "Username unique constraint failed"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    ),
                },
                Err(err) => {
                    let _ = txn.rollback().await;
                    to_response(
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "Failed to patch this resource"
                            })),
                        ),
                        Err(ApiError::DbError(err.to_string())),
                    )
                }
            }
        }
        Ok(None) => {
            let tmp = format!("No related entry with {} id was found.", id);
            to_response(
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": tmp
                    })),
                ),
                Err(ApiError::General(tmp)),
            )
        }
        Err(err) => to_response(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error."
                })),
            ),
            Err(ApiError::DbError(err.to_string())),
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
#[derive(Deserialize, Clone, Debug, Validate)]
struct RegisterUser {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: String,
    #[validate(regex(path = *PASSWORD_REGEX))]
    password: String,
}

#[derive(Deserialize, Clone, Debug, Validate)]
struct CreateUser {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: String,
    #[validate(regex(path = *PASSWORD_REGEX))]
    password: String,
    role: Role,
}

#[derive(Debug, Deserialize, Clone, Validate)]
struct UserLogin {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: String,
    #[validate(regex(path = *PASSWORD_REGEX))]
    password: String,
}

#[derive(Debug, Deserialize, Validate)]
struct PatchUser {
    role: Option<Role>,
    #[validate(regex(path = *USERNAME_REGEX))]
    username: Option<String>,
    #[validate(regex(path = *PASSWORD_REGEX))]
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

pub static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,25}$").unwrap());
static PASSWORD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9!@#$%^&*()_+]{8,15}$").unwrap());
