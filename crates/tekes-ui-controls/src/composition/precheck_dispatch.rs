//! Internal dispatch for host transaction prechecks, prepared effects, and focus evidence.

use std::any::Any;

use super::*;

fn downcast<T: Component>(component: &dyn Component) -> Option<&T> {
    (component as &dyn Any).downcast_ref::<T>()
}

fn downcast_mut<T: Component>(component: &mut dyn Component) -> Option<&mut T> {
    (component as &mut dyn Any).downcast_mut::<T>()
}

pub(crate) fn table_transaction_compatibility(
    component: &dyn Component,
) -> TableTransactionCompatibility {
    if downcast::<adapters::DialogContent>(component).is_some()
        || downcast::<Button>(component).is_some()
        || downcast::<combobox::ComboBoxContent>(component).is_some()
        || downcast::<datatable::RowWrapper>(component).is_some()
        || downcast::<icon::IconContent>(component).is_some()
        || downcast::<key_value_table::KeyValueTableContent>(component).is_some()
        || downcast::<panel::PanelContent>(component).is_some()
        || downcast::<search::SearchFieldContent>(component).is_some()
        || downcast::<selectable_list::ListRowSlot>(component).is_some()
        || downcast::<selectable_list::KeyedSelectableListContent>(component).is_some()
        || downcast::<separator::SeparatorContent>(component).is_some()
        || downcast::<split_view::PaneSlot<true>>(component).is_some()
        || downcast::<split_view::PaneSlot<false>>(component).is_some()
        || downcast::<split_view::OriginalDivider>(component).is_some()
        || downcast::<split_view::SplitViewContent>(component).is_some()
        || downcast::<text_field::TextFieldContent>(component).is_some()
        || downcast::<StackContent>(component).is_some()
        || downcast::<GridContent>(component).is_some()
        || downcast::<ScrollContent>(component).is_some()
        || downcast::<TextContent>(component).is_some()
    {
        return TableTransactionCompatibility::StableHooks;
    }
    if let Some(rich) = downcast::<rich_content::RichContent>(component) {
        return table_transaction_compatibility(rich.owner_field());
    }
    if let Some(md) = downcast::<markdown_editor::MarkdownEditorContent>(component) {
        return table_transaction_compatibility(md.owner_field());
    }
    if downcast::<adapters::ListContent>(component).is_some()
        || downcast::<chart_content::ChartContent>(component).is_some()
        || downcast::<checkbox::CheckboxContent>(component).is_some()
        || downcast::<datatable::DataTableContent>(component).is_some()
        || downcast::<menu::MenuContent>(component).is_some()
        || downcast::<popover::PopoverContent>(component).is_some()
        || downcast::<radio::RadioGroupContent>(component).is_some()
        || downcast::<segmented::SegmentedControlContent>(component).is_some()
        || downcast::<select::SelectContent>(component).is_some()
        || downcast::<toolbar_content::ToolbarGroup>(component).is_some()
        || downcast::<toolbar_content::ToolbarContent>(component).is_some()
    {
        return TableTransactionCompatibility::Unknown;
    }
    #[cfg(feature = "bitmap-images")]
    if downcast::<media_viewer::MediaViewerContent>(component).is_some() {
        return TableTransactionCompatibility::Unknown;
    }
    if let Some(inner) = component.delegated_owner() {
        return table_transaction_compatibility(inner);
    }
    if component.has_stable_hooks() {
        TableTransactionCompatibility::StableHooks
    } else {
        TableTransactionCompatibility::Unknown
    }
}

pub(crate) fn prepared_effects_work(
    component: &dyn Component,
    max_calls: usize,
) -> Result<usize, PreparedEffectsError> {
    if downcast::<adapters::DialogContent>(component).is_some()
        || downcast::<Button>(component).is_some()
        || downcast::<icon::IconContent>(component).is_some()
        || downcast::<key_value_table::KeyValueTableContent>(component).is_some()
        || downcast::<panel::PanelContent>(component).is_some()
        || downcast::<separator::SeparatorContent>(component).is_some()
        || downcast::<split_view::PaneSlot<true>>(component).is_some()
        || downcast::<split_view::PaneSlot<false>>(component).is_some()
        || downcast::<split_view::OriginalDivider>(component).is_some()
        || downcast::<split_view::SplitViewContent>(component).is_some()
        || downcast::<StackContent>(component).is_some()
        || downcast::<GridContent>(component).is_some()
    {
        return PreparedEffects::local_work(max_calls);
    }
    if let Some(combo) = downcast::<combobox::ComboBoxContent>(component) {
        return combo.combo_effects_work(max_calls);
    }
    if let Some(rich) = downcast::<rich_content::RichContent>(component) {
        return prepared_effects_work(rich.owner_field(), max_calls);
    }
    if let Some(md) = downcast::<markdown_editor::MarkdownEditorContent>(component) {
        return prepared_effects_work(md.owner_field(), max_calls);
    }
    if let Some(search) = downcast::<search::SearchFieldContent>(component) {
        return search.search_effects_work(max_calls);
    }
    if let Some(field) = downcast::<text_field::TextFieldContent>(component) {
        return field.prepare_field_effects_work(max_calls);
    }
    if let Some(scroll) = downcast::<ScrollContent>(component) {
        return scroll.scroll_effects_work(max_calls);
    }
    if let Some(text) = downcast::<TextContent>(component) {
        return text.text_effects_work(max_calls);
    }
    if downcast::<adapters::ListContent>(component).is_some()
        || downcast::<chart_content::ChartContent>(component).is_some()
        || downcast::<checkbox::CheckboxContent>(component).is_some()
        || downcast::<datatable::RowWrapper>(component).is_some()
        || downcast::<datatable::DataTableContent>(component).is_some()
        || downcast::<menu::MenuContent>(component).is_some()
        || downcast::<popover::PopoverContent>(component).is_some()
        || downcast::<radio::RadioGroupContent>(component).is_some()
        || downcast::<segmented::SegmentedControlContent>(component).is_some()
        || downcast::<select::SelectContent>(component).is_some()
        || downcast::<selectable_list::ListRowSlot>(component).is_some()
        || downcast::<selectable_list::KeyedSelectableListContent>(component).is_some()
        || downcast::<toolbar_content::ToolbarGroup>(component).is_some()
        || downcast::<toolbar_content::ToolbarContent>(component).is_some()
    {
        return Err(PreparedEffectsError::Unavailable);
    }
    #[cfg(feature = "bitmap-images")]
    if downcast::<media_viewer::MediaViewerContent>(component).is_some() {
        return Err(PreparedEffectsError::Unavailable);
    }
    if let Some(inner) = component.delegated_owner() {
        return prepared_effects_work(inner, max_calls);
    }
    if component.has_stable_hooks() && component.supports_prepared_lifecycle() {
        PreparedEffects::local_work(max_calls)
    } else {
        Err(PreparedEffectsError::Unavailable)
    }
}

pub(crate) fn prepare_effects(
    component: &dyn Component,
    hooks: &[PreparedHook],
) -> Result<PreparedEffects, PreparedEffectsError> {
    if let Some(dialog) = downcast::<adapters::DialogContent>(component) {
        return dialog.prepare_dialog_effects(hooks);
    }
    if let Some(button) = downcast::<Button>(component) {
        return button.prepare_button_effects(hooks);
    }
    if let Some(combo) = downcast::<combobox::ComboBoxContent>(component) {
        return combo.prepare_combo_effects(hooks);
    }
    if let Some(rich) = downcast::<rich_content::RichContent>(component) {
        return prepare_effects(rich.owner_field(), hooks);
    }
    if let Some(md) = downcast::<markdown_editor::MarkdownEditorContent>(component) {
        return prepare_effects(md.owner_field(), hooks);
    }
    if let Some(search) = downcast::<search::SearchFieldContent>(component) {
        return search.prepare_search_effects(hooks);
    }
    if let Some(field) = downcast::<text_field::TextFieldContent>(component) {
        return field.prepare_field_effects(hooks);
    }
    if let Some(scroll) = downcast::<ScrollContent>(component) {
        return scroll.prepare_scroll_effects(hooks);
    }
    if let Some(split) = downcast::<split_view::SplitViewContent>(component) {
        if !split.intact() {
            return Err(PreparedEffectsError::Unavailable);
        }
        return Ok(PreparedEffects::local(component, hooks));
    }
    if downcast::<icon::IconContent>(component).is_some()
        || downcast::<key_value_table::KeyValueTableContent>(component).is_some()
        || downcast::<panel::PanelContent>(component).is_some()
        || downcast::<separator::SeparatorContent>(component).is_some()
        || downcast::<split_view::PaneSlot<true>>(component).is_some()
        || downcast::<split_view::PaneSlot<false>>(component).is_some()
        || downcast::<split_view::OriginalDivider>(component).is_some()
        || downcast::<StackContent>(component).is_some()
        || downcast::<GridContent>(component).is_some()
        || downcast::<TextContent>(component).is_some()
    {
        return Ok(PreparedEffects::local(component, hooks));
    }
    if downcast::<adapters::ListContent>(component).is_some()
        || downcast::<chart_content::ChartContent>(component).is_some()
        || downcast::<checkbox::CheckboxContent>(component).is_some()
        || downcast::<datatable::RowWrapper>(component).is_some()
        || downcast::<datatable::DataTableContent>(component).is_some()
        || downcast::<menu::MenuContent>(component).is_some()
        || downcast::<popover::PopoverContent>(component).is_some()
        || downcast::<radio::RadioGroupContent>(component).is_some()
        || downcast::<segmented::SegmentedControlContent>(component).is_some()
        || downcast::<select::SelectContent>(component).is_some()
        || downcast::<selectable_list::ListRowSlot>(component).is_some()
        || downcast::<selectable_list::KeyedSelectableListContent>(component).is_some()
        || downcast::<toolbar_content::ToolbarGroup>(component).is_some()
        || downcast::<toolbar_content::ToolbarContent>(component).is_some()
    {
        return Err(PreparedEffectsError::Unavailable);
    }
    #[cfg(feature = "bitmap-images")]
    if downcast::<media_viewer::MediaViewerContent>(component).is_some() {
        return Err(PreparedEffectsError::Unavailable);
    }
    if let Some(inner) = component.delegated_owner() {
        return prepare_effects(inner, hooks);
    }
    if !component.has_stable_hooks() || !component.supports_prepared_lifecycle() {
        return Err(PreparedEffectsError::Unavailable);
    }
    let window_active_transitions = hooks
        .iter()
        .filter(|h| matches!(h.kind, PreparedHookKind::WindowActive(_)))
        .count();
    if !component.can_accept_window_active_transitions(window_active_transitions) {
        return Err(PreparedEffectsError::CounterCapacity);
    }
    Ok(PreparedEffects::local(component, hooks))
}

pub(crate) fn lifecycle_prepared(
    component: &mut dyn Component,
    event: Lifecycle,
    context: &mut PreparedLifecycleContext,
) {
    if let Some(scroll) = downcast_mut::<ScrollContent>(component) {
        context.scroll_lifecycle(scroll, event);
        return;
    }
    if let Some(rich) = downcast_mut::<rich_content::RichContent>(component) {
        lifecycle_prepared(rich.owner_field_mut(), event, context);
        return;
    }
    if let Some(md) = downcast_mut::<markdown_editor::MarkdownEditorContent>(component) {
        lifecycle_prepared(md.owner_field_mut(), event, context);
        return;
    }
    if let Some(inner) = component.delegated_owner_mut() {
        lifecycle_prepared(inner, event, context);
        return;
    }
    context.original_lifecycle(component, event);
}

pub(crate) fn action_transaction_compatibility(
    component: &dyn Component,
) -> ActionTransactionCompatibility {
    if downcast::<adapters::ListContent>(component).is_some()
        || downcast::<adapters::DialogContent>(component).is_some()
        || downcast::<Button>(component).is_some()
        || downcast::<chart_content::ChartContent>(component).is_some()
        || downcast::<checkbox::CheckboxContent>(component).is_some()
        || downcast::<combobox::ComboBoxContent>(component).is_some()
        || downcast::<datatable::DataTableContent>(component).is_some()
        || downcast::<key_value_table::KeyValueTableContent>(component).is_some()
        || downcast::<menu::MenuContent>(component).is_some()
        || downcast::<panel::PanelContent>(component).is_some()
        || downcast::<popover::PopoverContent>(component).is_some()
        || downcast::<radio::RadioGroupContent>(component).is_some()
        || downcast::<search::SearchFieldContent>(component).is_some()
        || downcast::<segmented::SegmentedControlContent>(component).is_some()
        || downcast::<select::SelectContent>(component).is_some()
        || downcast::<selectable_list::KeyedSelectableListContent>(component).is_some()
        || downcast::<split_view::PaneSlot<true>>(component).is_some()
        || downcast::<split_view::PaneSlot<false>>(component).is_some()
        || downcast::<split_view::OriginalDivider>(component).is_some()
        || downcast::<split_view::SplitViewContent>(component).is_some()
        || downcast::<text_field::TextFieldContent>(component).is_some()
        || downcast::<toolbar_content::ToolbarGroup>(component).is_some()
        || downcast::<toolbar_content::ToolbarContent>(component).is_some()
        || downcast::<StackContent>(component).is_some()
        || downcast::<GridContent>(component).is_some()
        || downcast::<ScrollContent>(component).is_some()
        || downcast::<TextContent>(component).is_some()
    {
        return ActionTransactionCompatibility::StableHooks;
    }
    #[cfg(feature = "bitmap-images")]
    if downcast::<media_viewer::MediaViewerContent>(component).is_some() {
        return ActionTransactionCompatibility::StableHooks;
    }
    if downcast::<icon::IconContent>(component).is_some()
        || downcast::<separator::SeparatorContent>(component).is_some()
        || downcast::<datatable::RowWrapper>(component).is_some()
        || downcast::<selectable_list::ListRowSlot>(component).is_some()
        || downcast::<rich_content::RichContent>(component).is_some()
        || downcast::<markdown_editor::MarkdownEditorContent>(component).is_some()
    {
        return ActionTransactionCompatibility::Unsupported;
    }
    if component.has_stable_hooks() || component.ignores_pointer_actions() {
        ActionTransactionCompatibility::StableHooks
    } else {
        ActionTransactionCompatibility::Unsupported
    }
}

pub(crate) fn prepare_action(
    component: &dyn Component,
    request: ComponentActionRequest<'_>,
) -> ActionPreparation {
    if downcast::<adapters::ListContent>(component).is_some()
        || downcast::<adapters::DialogContent>(component).is_some()
        || downcast::<key_value_table::KeyValueTableContent>(component).is_some()
        || downcast::<panel::PanelContent>(component).is_some()
        || downcast::<popover::PopoverContent>(component).is_some()
        || downcast::<split_view::PaneSlot<true>>(component).is_some()
        || downcast::<split_view::PaneSlot<false>>(component).is_some()
        || downcast::<split_view::SplitViewContent>(component).is_some()
        || downcast::<toolbar_content::ToolbarGroup>(component).is_some()
        || downcast::<toolbar_content::ToolbarContent>(component).is_some()
        || downcast::<StackContent>(component).is_some()
        || downcast::<GridContent>(component).is_some()
    {
        return action_preparation::ignored_pointer(request);
    }
    if let Some(button) = downcast::<Button>(component) {
        return button.prepare_button_action(request);
    }
    if let Some(chart) = downcast::<chart_content::ChartContent>(component) {
        return chart.prepare_chart_action(request);
    }
    if let Some(checkbox) = downcast::<checkbox::CheckboxContent>(component) {
        return checkbox.prepare_checkbox_action(request);
    }
    if let Some(combo) = downcast::<combobox::ComboBoxContent>(component) {
        return combo.prepare_combo_action(request);
    }
    if let Some(table) = downcast::<datatable::DataTableContent>(component) {
        return table.prepare_table_action(request);
    }
    #[cfg(feature = "bitmap-images")]
    if let Some(viewer) = downcast::<media_viewer::MediaViewerContent>(component) {
        return viewer.prepare_viewer_action(request);
    }
    if let Some(menu) = downcast::<menu::MenuContent>(component) {
        return menu.prepare_trigger_pointer(request);
    }
    if let Some(radio) = downcast::<radio::RadioGroupContent>(component) {
        return radio.prepare_radio_action(request);
    }
    if let Some(search) = downcast::<search::SearchFieldContent>(component) {
        return search.prepare_search_action(request);
    }
    if let Some(segmented) = downcast::<segmented::SegmentedControlContent>(component) {
        return segmented.prepare_segmented_action(request);
    }
    if let Some(select) = downcast::<select::SelectContent>(component) {
        return select.prepare_original_select(request);
    }
    if let Some(list) = downcast::<selectable_list::KeyedSelectableListContent>(component) {
        return list.prepare_keyed_list_action(request);
    }
    if let Some(divider) = downcast::<split_view::OriginalDivider>(component) {
        return divider.prepare_divider_action(request);
    }
    if let Some(field) = downcast::<text_field::TextFieldContent>(component) {
        return field.prepare_field_action(request);
    }
    if let Some(scroll) = downcast::<ScrollContent>(component) {
        return scroll.prepare_scroll_action(request);
    }
    if downcast::<TextContent>(component).is_some()
        || downcast::<icon::IconContent>(component).is_some()
        || downcast::<separator::SeparatorContent>(component).is_some()
        || downcast::<datatable::RowWrapper>(component).is_some()
        || downcast::<selectable_list::ListRowSlot>(component).is_some()
        || downcast::<rich_content::RichContent>(component).is_some()
        || downcast::<markdown_editor::MarkdownEditorContent>(component).is_some()
    {
        return ActionPreparation::Unsupported;
    }
    if component.ignores_pointer_actions() {
        action_preparation::ignored_pointer(request)
    } else {
        ActionPreparation::Unsupported
    }
}

pub(crate) fn required_focus_evidence(component: &dyn Component) -> Option<RequiredFocusEvidence> {
    if let Some(chart) = downcast::<chart_content::ChartContent>(component) {
        return chart.focus_evidence();
    }
    #[cfg(feature = "bitmap-images")]
    if let Some(viewer) = downcast::<media_viewer::MediaViewerContent>(component) {
        return viewer.focus_evidence();
    }
    #[cfg(test)]
    if let Some(evidence) = chart_content::witness_probe_evidence(component) {
        return Some(evidence);
    }
    #[cfg(test)]
    if let Some(probe) = downcast::<content_reveal_plan::EvidenceProbe>(component) {
        return Some(probe.evidence.clone());
    }
    None
}
