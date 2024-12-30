use sea_orm::entity::prelude::*;

use argon2::{
    password_hash::PasswordVerifier, 
    Argon2, 
    PasswordHash,
};

//use crate::entity::jwt_token;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub username: String,
    pub password: String,
}

impl Model {
    pub fn check_hash(&self, password: &str) -> Result<(), String> {
        let parsed_hash = PasswordHash::new(&self.password).unwrap();

        let argon2 = Argon2::default();
        argon2.verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| "Password verification failed")?;

        Ok(())
    }
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}