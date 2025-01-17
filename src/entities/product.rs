use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::category::Entity as Category;
use crate::entities::image::Entity as Image;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub price: f32,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub image_id: i32,
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
    )]
    Category,
    #[sea_orm(
        belongs_to = "Image",
        from = "crate::entities::product::Column::ImageId",
        to = "crate::entities::image::Column::Id",
    )]
    Image,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<crate::entities::category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Category.def()
    }
}

impl Related<crate::entities::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}
