use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::image::Entity as Image;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub image_id: i32,
    #[sea_orm(default = false)]
    pub is_featured: bool,
    #[sea_orm(default = true)]
    pub is_available: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "Image",
        from = "crate::entities::category::Column::ImageId",
        to = "crate::entities::image::Column::Id",
        on_update = "Cascade",
    )]
    Image,
}

impl ActiveModelBehavior for ActiveModel {}
