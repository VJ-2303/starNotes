use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "card")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub board_id: i64,
    pub position: i32,
    pub workflow_role: String,
    pub deleted_at: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
