//! A complete synchronous caller update: preflight the original editor, then
//! validate the reading token, then change transcript and editor in one turn.
//! No editor clone/replacement, copied validation, pixel offset or Agent types.
use crate::scroll_updates::{COMPOSER, ITEMS, SCROLLER, Scenario};
use tekes_ui_controls::{
    DraftRevision, ExternalSelection as S, ExternalUndo as U, ExternalValueConflict,
    ExternalValueResult, ReadingDocumentVersion, ReadingUpdateError, TextEditBaseline,
    composition::*,
};
#[derive(Debug, PartialEq, Eq)]
pub enum PublicationError {
    Editor(ExternalValueConflict),
    Reading(ReadingUpdateError),
    Structure,
}
/// The fixture's model/tree preconditions are checked before entering the
/// reading callback. Do not yield, dispatch events or invoke user callbacks
/// between preflight and apply. A check is not a cross-turn reservation.
pub fn publish(
    scenario: &mut Scenario,
    reading: &ReadingDocumentVersion,
    baseline: &TextEditBaseline,
    draft: &DraftRevision,
    row: ComponentId,
) -> Result<ReadingDocumentVersion, PublicationError> {
    let field = scenario
        .host
        .get(COMPOSER)
        .and_then(|n| n.component::<TextFieldContent>())
        .ok_or(PublicationError::Structure)?;
    let predicted =
        field
            .surface
            .check_external_value(baseline, draft, S::Preserve, U::Transaction);
    if let ExternalValueResult::Conflict(conflict) = predicted {
        return Err(PublicationError::Editor(conflict));
    }
    if scenario.host.get(row).is_some()
        || scenario
            .host
            .get(ITEMS)
            .and_then(|n| n.component::<ListContent>())
            .is_none()
    {
        return Err(PublicationError::Structure);
    }
    let mut order = scenario.item_keys();
    order.push(row);
    let revision = reading
        .revision()
        .checked_add(1)
        .ok_or(PublicationError::Reading(
            ReadingUpdateError::InvalidDocument,
        ))?;
    let content = Content::new(
        row,
        TextContent::new(format!("Publication · {}", draft.payload)),
    );
    let (version, ()) = scenario
        .host
        .update_scroll_reading_document(SCROLLER, reading, revision, order.clone(), |host| {
            let field = host
                .get_mut(COMPOSER)
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap();
            assert_eq!(
                field
                    .surface
                    .apply_external_value(baseline, draft, S::Preserve, U::Transaction),
                predicted,
                "same-turn preflight must not be separated by mutations/events"
            );
            // All structural failures were checked above; no arbitrary callbacks here.
            host.insert(ITEMS, content).unwrap();
            host.reorder(ITEMS, &order).unwrap();
        })
        .map_err(PublicationError::Reading)?;
    Ok(version)
}
