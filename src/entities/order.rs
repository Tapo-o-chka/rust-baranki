use crate::entities::user::Entity as User;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "order")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub status: Status,
    pub user_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "User",
        from = "Column::UserId",
        to = "crate::entities::user::Column::Id"
    )]
    User,
}
impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Copy, PartialEq, Debug, EnumIter, DeriveActiveEnum, Serialize)]
#[sea_orm(
    enum_name = "status_enum",
    db_type = "String(StringLen::N(255))",
    rs_type = "String"
)]

pub enum Status {
    #[sea_orm(string_value = "created")]
    Created,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "arriving")]
    Arriving,
    #[sea_orm(string_value = "waiting")]
    Waiting,
    #[sea_orm(string_value = "received")]
    Received,
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "processing" => Ok(Self::Processing),
            "arriving" => Ok(Self::Arriving),
            "waiting" => Ok(Self::Waiting),
            "received" => Ok(Self::Received),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Self::Created => "created".to_string(),
            Self::Processing => "processing".to_string(),
            Self::Arriving => "arriving".to_string(),
            Self::Waiting => "Waiting".to_string(),
            Self::Received => "Received".to_string(),
        }
    }
}

impl Related<crate::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}
