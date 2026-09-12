#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueReminder {
    pub entry_id: i64,
    pub title: String,
    pub due_on: String,
    pub board_title: String,
    pub list_title: String,
}

pub async fn load_due_reminders(
    _db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    _due_through: &str,
) -> anyhow::Result<Vec<DueReminder>> {
    Ok(Vec::new())
}

pub async fn mark_reminder_notified(
    _db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    _entry_id: i64,
    _due_on: String,
) -> anyhow::Result<()> {
    Ok(())
}

pub async fn mark_many_reminders_notified(
    _db: &(
         impl sea_orm::ConnectionTrait
         + sea_orm::TransactionTrait<Transaction = sea_orm::DatabaseTransaction>
     ),
    _reminders: &[(i64, String)],
) -> anyhow::Result<()> {
    Ok(())
}
