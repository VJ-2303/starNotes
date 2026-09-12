use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "entry_label")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub entry_id: i64,
    pub board_label_id: i64,
}

impl ActiveModelBehavior for ActiveModel {}
