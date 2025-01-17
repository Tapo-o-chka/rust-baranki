use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    middleware,
    response::Response,
    routing::{get, patch},
    Json, Router,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::entities::user::Role;
use crate::entities::{cart, cart::Entity as CartEntity, category, product, user};
use crate::middleware::{
    auth::{auth_middleware, Claims},
    logging::{to_response, ApiError},
};

//ROUTERS
pub fn cart_routes() -> Router {
    Router::new()
        .route("/cart", get(get_cart).post(add_product))
        .route("/cart/:id", patch(patch_entry).delete(remove_product))
        .layer(middleware::from_fn_with_state(Role::User, auth_middleware))
}

pub fn admin_cart_routes() -> Router {
    Router::new()
        .route("/cart", get(get_carts))
        .route(
            "/cart:id",
            patch(admin_remove_product).post(admin_patch_cart_entry),
        )
        .layer(middleware::from_fn_with_state(Role::Admin, auth_middleware))
}

//Routes
async fn get_cart(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<CartQuery>,
) -> Response {
    let user_id = claims.user_id;
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

    let mut condition = Condition::all().add(cart::Column::UserId.eq(user_id));

    //Filter zone
    if let Some(price_bottom) = query.price_bottom {
        condition = condition.add(product::Column::Price.gte(price_bottom));
    }
    if let Some(price_top) = query.price_top {
        condition = condition.add(product::Column::Price.lte(price_top));
    }
    if let Some(category_ids) = query.category_ids {
        condition = condition.add(product::Column::CategoryId.is_in(category_ids));
    }
    if query.only_available.unwrap_or(false) {
        condition = condition.add(product::Column::IsAvailable.eq(true));
    }
    if query.only_featured.unwrap_or(false) {
        condition = condition.add(product::Column::IsFeatured.eq(true))
    }

    //Sorting zone
    let order = match query.order.as_deref() {
        Some("desc") => sea_orm::Order::Desc,
        _ => sea_orm::Order::Asc,
    };

    let sort_column = match query.sort_by.as_deref() {
        Some("price") => product::Column::Price,
        Some("availability") => product::Column::IsAvailable,
        _ => product::Column::Name,
    };

    let sort_cart_column = match query.sort_by.as_deref() {
        Some("quantity") => Some(cart::Column::Quantity),
        _ => None,
    };

    condition = condition.add(category::Column::IsAvailable.eq(true));

    //Pagination zone
    let page: u64 = query.page.unwrap_or(1);
    let page_size: u64 = query.page_size.unwrap_or(10);

    let mut half_items = cart::Entity::find()
        .filter(condition)
        .join(JoinType::InnerJoin, cart::Relation::Product.def())
        .join(JoinType::InnerJoin, product::Relation::Category.def())
        .column_as(product::Column::Id, "product_id")
        .column_as(product::Column::Name, "name")
        .column_as(product::Column::Price, "price")
        .column_as(product::Column::ImageId, "image_id")
        .column_as(category::Column::Name, "category_name")
        .column_as(product::Column::IsAvailable, "is_available");

    //hate this, but let it be
    if let Some(col) = sort_cart_column {
        half_items = half_items.order_by(col, order)
    } else {
        half_items = half_items.order_by(sort_column, order)
    }

    if let Some(query) = query.query {
        half_items =
            half_items.filter(Condition::any().add(product::Column::Name.contains(query.clone())));
        //ahh adding filter after column definitions and ordering, hate that
    }

    let items = half_items
        .limit(page_size)
        .offset((page - 1) * page_size)
        .into_model::<CartResponse>()
        .all(&txn)
        .await
        .unwrap_or_else(|_| vec![]);

    to_response(Json(items), Ok(()))
}

async fn add_product(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AddProduct>,
) -> Response {
    let user_id = claims.user_id;
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

    //Nested as hell
    match product::Entity::find_by_id(payload.product_id)
        .one(&txn)
        .await
    {
        Ok(Some(_)) => {
            if payload.quantity > 0 {
                //If entry already exist in db, so we would expand it, instead of creating second one.
                if let Ok(Some(entry)) = CartEntity::find()
                    .filter(cart::Column::ProductId.eq(payload.product_id))
                    .filter(cart::Column::UserId.eq(user_id))
                    .one(&txn)
                    .await
                {
                    let mut entry: cart::ActiveModel = entry.into();
                    entry.quantity = Set(entry.quantity.unwrap() + payload.quantity);
                    let result = entry.update(&txn).await.map(|_| ());
                    match result {
                        Ok(_) => {
                            return match txn.commit().await {
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
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(json!({
                                            "error": "Internal server error"
                                        })),
                                    ),
                                    Err(ApiError::DbError(err.to_string())),
                                ),
                            };
                        }
                        Err(err) => {
                            let _ = txn.rollback().await;
                            return to_response(
                                (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!({
                                        "error": "Failed to patch this resource"
                                    })),
                                ),
                                Err(ApiError::DbError(err.to_string())),
                            );
                        }
                    };
                };
                let new_entry = cart::ActiveModel {
                    user_id: Set(user_id),
                    product_id: Set(payload.product_id),
                    quantity: Set(payload.quantity),
                    ..Default::default()
                };
                match CartEntity::insert(new_entry).exec(&txn).await {
                    Ok(_) => match txn.commit().await {
                        Ok(_) => to_response(
                            (
                                StatusCode::CREATED,
                                Json(json!({
                                    "message": "Added successfully"
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
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Internal server error"
                                })),
                            ),
                            Err(ApiError::DbError(err.to_string())),
                        )
                    }
                }
            } else {
                let tmp = "Quantity should be greater than 0".to_owned();
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
        }
        Ok(None) => {
            let tmp = format!("No product with {} id was found", payload.product_id);
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

async fn remove_product(
    Path(id): Path<i32>,
    Extension(claims): Extension<Claims>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Response {
    let user_id = claims.user_id;
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

    match CartEntity::find_by_id(id)
        .filter(cart::Column::UserId.eq(user_id))
        .one(&txn)
        .await
    {
        Ok(Some(entry)) => {
            let entry: cart::ActiveModel = entry.into();
            match entry.delete(&txn).await {
                Ok(_) => {
                    if txn.commit().await.is_ok() {
                        to_response(
                            (
                                StatusCode::OK,
                                Json(json!({
                                    "message": "Resource deleted successfully"
                                })),
                            ),
                            Ok(()),
                        )
                    } else {
                        to_response(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "error": "Failed to commit transaction"
                                })),
                            ),
                            Err(ApiError::TransactionCreationFailed),
                        )
                    }
                }
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
        Ok(None) => to_response(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("No related entry with {} id was found.", id)
                })),
            ),
            Err(ApiError::ValidationFail(format!(
                "No entry found for id {}",
                id
            ))),
        ),
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

async fn patch_entry(
    Path(id): Path<i32>,
    Extension(claims): Extension<Claims>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchCart>,
) -> Response {
    let user_id = claims.user_id;
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

    match CartEntity::find_by_id(id)
        .filter(cart::Column::UserId.eq(user_id))
        .one(&txn)
        .await
    {
        Ok(Some(entry)) => {
            let mut entry: cart::ActiveModel = entry.into();

            let result: Result<(), DbErr> = match payload.quantity {
                value if value == 0 => entry.delete(&txn).await.map(|_| ()),
                _ => {
                    entry.quantity = Set(payload.quantity);
                    entry.update(&txn).await.map(|_| ())
                }
            };
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

async fn get_carts(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(query): Query<AdminCartsQuery>,
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

    let users = match user_finder.order_by(sort_users, order).all(&txn).await {
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

    let mut user_cart_list = Vec::new();

    for user in users {
        let condition = Condition::all().add(cart::Column::UserId.eq(user.id));

        let cart_items = match CartEntity::find()
            .filter(condition)
            .select_only() //to select specific columns
            .column_as(cart::Column::Id, "id")
            .column_as(cart::Column::Quantity, "quantity")
            .column_as(product::Column::Id, "product_id")
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::Price, "product_price")
            .column_as(product::Column::IsAvailable, "is_available")
            .column_as(category::Column::Id, "category_id")
            .column_as(category::Column::Name, "category_name")
            .join(JoinType::InnerJoin, cart::Relation::Product.def())
            .join(JoinType::InnerJoin, product::Relation::Category.def())
            .into_model::<PrepareCartItem>()
            .all(&txn)
            .await
        {
            Ok(result) => result,
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

        let mut cart: Vec<CartItem> = Vec::new();
        let mut total = 0.0;
        let mut total_available = 0.0;
        let mut total_quantity: u32 = 0;
        for item in cart_items {
            let total_price = item.quantity as f64 * item.product_price;
            if item.is_available {
                total_available += total_price;
            }
            total += total_price;
            total_quantity += item.quantity;

            if let (Some(total_entries_bottom), Some(total_entries_top)) =
                (query.total_entries_bottom, query.total_entries_top)
            {
                if total_entries_bottom > total_quantity || total_entries_top < total_quantity {
                    continue;
                }
            }

            cart.push(CartItem {
                id: item.id,
                product: ProductItem {
                    id: item.product_id,
                    name: item.product_name,
                    price: item.product_price,
                },
                category: CategoryItem {
                    id: item.category_id,
                    name: item.category_name,
                },
                quantity: item.quantity,
                price: total_price,
                is_available: item.is_available,
            });
        }

        if query.non_empty.unwrap_or(true) {
            continue;
        }

        if let (Some(cart_total_bottom), Some(cart_total_top)) =
            (query.cart_total_bottom, query.cart_total_top)
        {
            if cart_total_bottom > total_available || cart_total_top < total_available {
                //makes sense to take total_available instead of total
                continue;
            }
        }

        user_cart_list.push(UsersEntry {
            id: user.id,
            role: user.role,
            cart,
            total,
            total_available,
        });
    }
    to_response(Json(user_cart_list), Ok(()))
}

async fn admin_remove_product(
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
            )
        }
    };

    match CartEntity::find_by_id(id).one(&txn).await {
        Ok(Some(entry)) => {
            let entry: cart::ActiveModel = entry.into();
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

async fn admin_patch_cart_entry(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<PatchCart>,
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

    match CartEntity::find_by_id(id).one(&txn).await {
        Ok(Some(entry)) => {
            let mut entry: cart::ActiveModel = entry.into();

            let result: Result<(), DbErr> = match payload.quantity {
                value if value == 0 => entry.delete(&txn).await.map(|_| ()),
                _ => {
                    entry.quantity = Set(payload.quantity);
                    entry.update(&txn).await.map(|_| ())
                }
            };
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
//Structs
#[derive(Deserialize, Debug)]
struct CartQuery {
    //Query
    query: Option<String>,
    //sort zone
    sort_by: Option<String>, //Enum better?? "price", "quantity", "availability", "name"
    order: Option<String>,   //Enum better??
    //filter zone
    price_top: Option<i32>,
    price_bottom: Option<i32>,
    category_ids: Option<Vec<i32>>,
    only_available: Option<bool>,
    only_featured: Option<bool>,
    //pagination zone
    page: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
    page_size: Option<u64>, //required by sea_orm to be u64, why? trait into u64 or something
}

#[derive(Deserialize)]
struct AdminCartsQuery {
    //Query
    query: Option<String>,
    //Sort zone
    sort_by: Option<String>, //Enum better?? "id", "username", "role"
    order: Option<String>,   //Enum better??
    //filter zone
    role: Option<Role>, //incoming should be None, "user" or "admin"
    non_empty: Option<bool>,
    cart_total_bottom: Option<f64>,
    cart_total_top: Option<f64>,
    total_entries_bottom: Option<u32>,
    total_entries_top: Option<u32>,
}

//Preparing response by admin_get_carts
#[derive(Debug, Deserialize, FromQueryResult)]
struct PrepareCartItem {
    id: i32,
    quantity: u32,
    product_id: i32,
    product_name: String,
    product_price: f64,
    is_available: bool,
    category_id: i32,
    category_name: String,
}

//Building Response by admin_get_carts
#[derive(Debug, Deserialize, Serialize)]
struct ProductItem {
    id: i32,
    name: String,
    price: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct CategoryItem {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CartItem {
    id: i32,
    product: ProductItem,
    category: CategoryItem,
    quantity: u32,
    price: f64,
    is_available: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct UsersEntry {
    id: i32,
    role: Role,
    cart: Vec<CartItem>,
    total: f64,
    total_available: f64,
}

#[derive(Deserialize, Debug)]
struct AddProduct {
    product_id: i32,
    quantity: u32, //maybe u16 is enough...
}

#[derive(Deserialize)]
struct PatchCart {
    quantity: u32,
}

#[derive(Serialize, FromQueryResult)]
struct CartResponse {
    product_id: i32,
    name: String,
    price: f64,
    image_id: i32,
    category_name: String,
    is_available: bool,
}
