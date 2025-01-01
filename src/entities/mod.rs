pub mod user;
pub mod product;
pub mod cart;
pub mod category;
pub mod image;
//pub mod roles; 

use sea_orm::{Schema, DatabaseConnection, ConnectionTrait};
use crate::entities::{
    cart::Entity as Crate,
    category::Entity as Category,
    user::Entity as User,
    product::Entity as Product,
    image::Entity as Picture,
};

pub async fn setup_schema(db: &DatabaseConnection) {
    let schema = Schema::new(db.get_database_backend());
    let create_cart_table = schema.create_table_from_entity(Crate);
    let create_category_table = schema.create_table_from_entity(Category);
    let create_user_table = schema.create_table_from_entity(User);
    let create_product_table = schema.create_table_from_entity(Product);
    let create_picture_table = schema.create_table_from_entity(Picture);

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
    db.execute(db.get_database_backend().build(&create_picture_table))
        .await
        .expect("Failed to create picture schema");
}