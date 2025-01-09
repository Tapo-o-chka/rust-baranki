use sea_orm::entity::prelude::*;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "image")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub file_name: String,
    #[sea_orm(unique)]
    pub path_name: String,
    pub extension: FileExtension
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Copy, PartialEq, Debug, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    enum_name = "extension_enum",
    db_type = "String(StringLen::N(255))",
    rs_type = "String"
)]
pub enum FileExtension {
    #[sea_orm(string_value = "jpg")]
    JPG,
    #[sea_orm(string_value = "png")]
    PNG,
}

impl FromStr for FileExtension {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpg" => Ok(FileExtension::JPG),
            "png" => Ok(FileExtension::PNG),
            _ => Err(()),
        }
    }
}

impl ToString for FileExtension {
    fn to_string(&self) -> String {
        match self {
            FileExtension::JPG => "jpg".to_string(),
            FileExtension::PNG => "png".to_string(),
        }
    }
}