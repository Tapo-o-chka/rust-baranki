use sea_orm::entity::prelude::*;
use argon2::{password_hash::PasswordVerifier, Argon2, PasswordHash};
use serde::{Serialize, Deserialize};
use std::str::FromStr;

//use crate::entity::jwt_token;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub username: String,
    pub password: String,
    pub role: Role,
}

impl Model {
    pub fn check_hash(&self, password: &str) -> Result<(), String> {
        let parsed_hash = match PasswordHash::new(&self.password){
            Ok(value) => value,
            Err(err) => panic!("Error: {err}")
        };

        let argon2 = Argon2::default();
        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| "Password verification failed")?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Copy, PartialEq, Debug, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(
    enum_name = "role_enum",
    db_type = "String(StringLen::N(255))",
    rs_type = "String"
)]
pub enum Role {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "user")]
    User,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err(()),
        }
    }
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::Admin => "admin".to_string(),
            Role::User => "user".to_string(),
        }
    }
}