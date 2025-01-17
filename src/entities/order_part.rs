use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "order_part")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub quantity: i32,
    pub product_id: i32,
    pub order_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::entities::product::Entity",
        from = "Column::ProductId",
        to = "crate::entities::product::Column::Id",
    )]
    Product,
    #[sea_orm(
        belongs_to = "crate::entities::order::Entity",
        from = "Column::OrderId",
        to = "crate::entities::order::Column::Id",
    )]
    Order,
}

impl Related<crate::entities::order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl Related<crate::entities::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
