use super::*;

impl<C> Store<C>
where
    C: ConnectionTrait + TransactionTrait + Send + Sync + 'static,
{
    pub(super) async fn related_items_for_note(
        &self,
        note_id: i64,
    ) -> Result<Vec<RelatedItemDetail>> {
        let links =
            crate::workspace::links::load_note_workspace_links(self.db.as_ref(), note_id).await?;
        let mut grouped = HashMap::<
            crate::workspace::links::WorkspaceItemRef,
            (crate::workspace::links::WorkspaceCatalogEntry, Vec<String>),
        >::new();
        for reference in links.references {
            let origin = workspace_origin_label(reference.origin);
            let row = grouped
                .entry(reference.item.item)
                .or_insert_with(|| (reference.item.clone(), Vec::new()));
            if !row.1.iter().any(|existing| existing == origin) {
                row.1.push(origin.to_string());
            }
        }
        let mut details = grouped
            .into_values()
            .map(|(entry, origins)| related_item_detail(entry, origins))
            .collect::<Vec<_>>();
        details.sort_by_key(|detail| (detail.kind.clone(), detail.breadcrumb.to_lowercase()));
        Ok(details)
    }
}
