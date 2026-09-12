use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "board_label")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub board_id: i64,
    pub name: String,
    pub color: String,
}

impl ActiveModelBehavior for ActiveModel {}
