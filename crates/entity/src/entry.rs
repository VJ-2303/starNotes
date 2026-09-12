use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "entry")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub description: String,
    pub card_id: i64,
    pub position: i32,
    pub due_on: Option<String>,
    pub deleted_at: Option<i64>,
    pub reminder_enabled: bool,
    pub reminder_notified_for: Option<String>,
    pub start_on: Option<String>,
    pub completed_at: Option<i64>,
    pub cancelled_at: Option<i64>,
    pub archived: bool,
}

impl ActiveModelBehavior for ActiveModel {}
