use sea_orm::entity::prelude::*;
use crate::entities::user::Entity as User;
use crate::entities::product::Entity as Product;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cart")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(indexed)]
    pub user_id: i32,
    pub item_id: i32,
    pub quantity: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "User",
        from = "crate::entities::cart::Column::UserId",
        to = "crate::entities::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "Product",
        from = "crate::entities::cart::Column::ItemId",
        to = "crate::entities::product::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Product,
}


impl ActiveModelBehavior for ActiveModel {}
