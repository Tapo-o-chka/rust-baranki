use sea_orm::entity::prelude::*;
use crate::entities::category::Entity as Category;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub price: f32,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub image_url: String,
    pub category_id: i32,
    #[sea_orm(default = false)]
    pub is_featured: bool,
    #[sea_orm(default = true)]
    pub is_available: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "Category",
        from = "crate::entities::product::Column::CategoryId",
        to = "crate::entities::category::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Category,
}

impl ActiveModelBehavior for ActiveModel {}
