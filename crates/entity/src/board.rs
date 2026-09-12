use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "board")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub project_id: Option<i64>,
    pub is_pinned: bool,
    pub last_opened_at: Option<i64>,
    pub last_selected_view_id: i64,
    pub deleted_at: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
