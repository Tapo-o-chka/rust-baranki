pub mod user;
pub mod product;
pub mod cart;
pub mod category;
pub mod image;
pub mod order;
pub mod order_part;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use std::sync::Arc;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, Schema, Set, TransactionTrait};
use crate::entities::{
    cart::Entity as Crate,
    category::Entity as Category,
    user::Entity as User,
    product::Entity as Product,
    image::Entity as Image,
};

pub async fn setup_schema(db: &DatabaseConnection) {
    let schema = Schema::new(db.get_database_backend());
    let create_cart_table = schema.create_table_from_entity(Crate);
    let create_category_table = schema.create_table_from_entity(Category);
    let create_user_table = schema.create_table_from_entity(User);
    let create_product_table = schema.create_table_from_entity(Product);
    let create_image_table = schema.create_table_from_entity(Image);

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
    db.execute(db.get_database_backend().build(&create_image_table))
        .await
        .expect("Failed to create image schema");
}

pub async fn primary_settup(db: Arc<DatabaseConnection>){
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password("Secret15".as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    let new_admin = user::ActiveModel {
        username: Set("admin".to_owned()),
        password: Set(password_hash.clone()),
        role: Set(user::Role::Admin),
        ..Default::default()
    };

    let new_user = user::ActiveModel {
        username: Set("user".to_owned()),
        password: Set(password_hash),
        role: Set(user::Role::User),
        ..Default::default()
    };

    match db.begin().await {
        Ok(txn) => {
            match user::Entity::insert_many([new_user, new_admin]).exec(&txn).await {
                Ok(_) => match txn.commit().await {
                    Ok(_) => {
                    },
                    Err(_) => {
                        panic!("Failed to pramary setup db, but function requested.");
                    }
                },
                Err(_) => {
                    let _ = txn.rollback().await;
                    panic!("Failed to pramary setup db, but function requested.");
                }
            }
        },
        Err(_) => {
            panic!("Failed to pramary setup db, but function requested.");
        }
    }
}