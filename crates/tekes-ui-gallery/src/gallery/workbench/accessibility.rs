use super::*;
use crate::gallery::accessibility::{DETAIL, INPUT, STORY, node};
use tekes_ui_platform_macos::{
    AccessibilityAction as A, AccessibilityKey as K, AccessibilityNode as N, AccessibilityRole as R,
};
impl Workbench {
    pub(in crate::gallery) fn search_native_layouts(
        &self,
        i: usize,
    ) -> Vec<(K, tekes_ui_platform_macos::AccessibilityTextLayout)> {
        self.states
            .get(&i)
            .and_then(|s| s.input.search_host())
            .map(|host| host.native_layouts())
            .unwrap_or_default()
    }

    pub(in crate::gallery) fn accessibility_modal(&self, i: usize) -> bool {
        self.states
            .get(&i)
            .is_some_and(|s| s.dialog_view.dialog().open)
    }
    pub(in crate::gallery) fn accessibility_nodes(&self, i: usize) -> Vec<N> {
        let Some(s) = self.states.get(&i) else {
            return Vec::new();
        };
        if s.dialog_view.dialog().open {
            let Some(layout) = s.dialog_layout.as_ref() else {
                return Vec::new();
            };
            let Some(frame) = layout.visible.and_then(|v| layout.modal.intersection(&v)) else {
                return Vec::new();
            };
            let root = K {
                component: STORY + i as u64,
                part: Some(69),
            };
            let mut nodes = vec![node(
                root,
                R::Dialog,
                StoryState::dialog_title(entry(i).renderer()),
                frame,
            )];
            for (index, button) in layout.buttons.iter().enumerate() {
                if let Some(frame) = button.visible {
                    let mut n = node(
                        K {
                            component: root.component,
                            part: Some(70 + index),
                        },
                        R::Button,
                        DialogView::button_label(index).unwrap(),
                        frame,
                    );
                    n.parent = Some(root);
                    n.enabled = !s.disabled;
                    n.focused = s.window_active && s.dialog_view.dialog().focus == index;
                    nodes.push(n);
                }
            }
            return nodes;
        }
        let renderer = entry(i).renderer();
        let mut nodes = if renderer == "select" {
            s.select_story.native_nodes()
        } else {
            Vec::new()
        };
        if renderer == "slider"
            && s.logical_focus.iter().any(|t| t.id == 20 && t.mounted)
            && let Some(frame) = s.extended.slider_view.layout().visible
        {
            let model = &s.extended.slider;
            let range = model.range();
            let mut n = node(
                K {
                    component: STORY + i as u64,
                    part: Some(20),
                },
                R::Slider(
                    tekes_ui_platform_macos::AccessibilityRange::new(*range.start(), *range.end())
                        .expect("valid Slider range"),
                ),
                "Value",
                frame,
            );
            n.value = Some(model.value().to_string());
            n.enabled = !s.disabled && !model.disabled;
            n.focused = s.window_active && s.focus == Some(20);
            nodes.push(n);
        }
        for target in &s.logical_focus {
            // The Select trigger/parts have their one ContentHost projection;
            // keep the Gallery's independently mounted background controls.
            if renderer == "select" && target.id == 20 {
                continue;
            }
            if !target.mounted {
                continue;
            }
            let Some(label) = s.button_labels.get(&target.id) else {
                continue;
            };
            let Some(button) = s.buttons.get(&target.id) else {
                continue;
            };
            let Some(layout) = button.layout() else {
                continue;
            };
            let Some(frame) = layout.visible else {
                continue;
            };
            let mut n = node(
                K {
                    component: STORY + i as u64,
                    part: Some(target.id),
                },
                R::Button,
                label,
                frame,
            );
            n.enabled = !(button.state.disabled || button.state.loading);
            n.focused = s.window_active && s.focus == Some(target.id);
            n.selected = button.state.selected;
            if renderer == "menu" && target.id == 20 {
                n.role = R::MenuButton;
                n.expanded = Some(s.menu_view.is_open());
            }
            if renderer == "popover" && target.id == 20 {
                n.selected = false;
                n.expanded = Some(s.extended.popover.open);
            }
            if renderer == "checkbox" && target.id == 20 {
                n.role = R::Checkbox;
                n.selected = s.checkbox.checked == CheckState::Checked;
                n.value = Some(
                    match s.checkbox.checked {
                        CheckState::Unchecked => "off",
                        CheckState::Checked => "on",
                        CheckState::Mixed => "mixed",
                    }
                    .to_owned(),
                );
            }
            nodes.push(n);
        }
        // These retained views own their models and layout outside `buttons`.
        // Reuse mounted focus eligibility and the actual paint/hit geometry.
        let feedback_surface = s.extended.feedback_surface(renderer);
        let feedback_parent = K {
            component: STORY + i as u64,
            part: Some(19),
        };
        if let Some((frame, label)) = &feedback_surface {
            let mut n = node(feedback_parent, R::Group, label, *frame);
            n.enabled = !s.disabled;
            nodes.push(n);
        }
        let mut choices = Vec::new();
        for target in s.logical_focus.iter().filter(|t| t.mounted) {
            let (role, label, selected) = match (renderer, target.id) {
                ("popover", 21) | ("toast" | "banner" | "empty-state", 20)
                    if feedback_surface.is_some()
                        && match renderer {
                            "popover" => s.extended.popover.open,
                            "toast" => s.extended.toast.visible,
                            "banner" => s.extended.banner.visible && s.extended.banner.dismissible,
                            "empty-state" => !s.extended.empty_state_created,
                            _ => false,
                        } =>
                {
                    (
                        R::Button,
                        s.button_labels
                            .get(&target.id)
                            .map(String::as_str)
                            .unwrap_or(""),
                        false,
                    )
                }
                ("link", 20) => (
                    R::Link,
                    s.button_labels.get(&20).map(String::as_str).unwrap_or(""),
                    false,
                ),
                ("icon-button", 20) => (
                    R::Button,
                    s.extended.icon_button_view.name(),
                    s.extended.icon_button_view.button().button.state.selected,
                ),
                ("switch", 20) => (
                    R::Switch,
                    s.button_labels.get(&20).map(String::as_str).unwrap_or(""),
                    s.extended.switch.checked,
                ),
                ("radio-group", id @ 30..=32) => (
                    R::RadioButton,
                    s.button_labels.get(&id).map(String::as_str).unwrap_or(""),
                    s.extended.radio.selected == Some(id - 30),
                ),
                ("list", id @ 30..=34) => (
                    R::ListItem,
                    s.button_labels.get(&id).map(String::as_str).unwrap_or(""),
                    s.extended.list.selected == Some(id - 30),
                ),
                ("segmented-control", id @ 30..=32) => (
                    R::RadioButton,
                    s.button_labels.get(&id).map(String::as_str).unwrap_or(""),
                    s.extended.segmented.selected == id - 30,
                ),
                _ => continue,
            };
            let frame = if renderer == "icon-button" {
                s.extended.icon_button_view.layout().visible
            } else {
                s.hits
                    .iter()
                    .find(|(id, _)| *id == target.id)
                    .map(|(_, frame)| *frame)
            };
            let Some(frame) = frame else {
                continue;
            };
            let mut n = node(
                K {
                    component: STORY + i as u64,
                    part: Some(target.id),
                },
                role,
                label,
                frame,
            );
            n.enabled = target.eligible;
            n.focused = s.window_active && s.focus == Some(target.id);
            n.selected = selected;
            if matches!(
                (renderer, target.id),
                ("popover", 21) | ("toast" | "banner" | "empty-state", 20)
            ) {
                n.parent = Some(feedback_parent);
            }
            if matches!(role, R::RadioButton | R::ListItem) {
                choices.push(n);
            } else {
                nodes.push(n);
            }
        }
        if let Some(first) = choices.first() {
            let mut group = first.clone();
            group.key.part = Some(29);
            group.role = if renderer == "list" {
                R::List
            } else {
                R::RadioGroup
            };
            group.label = entry(i).title().to_owned();
            group.selected = false;
            group.focused = false;
            group.enabled = choices.iter().any(|n| n.enabled);
            for choice in &mut choices {
                let x = group.frame.origin.x.min(choice.frame.origin.x);
                let y = group.frame.origin.y.min(choice.frame.origin.y);
                let right = (group.frame.origin.x + group.frame.size.width)
                    .max(choice.frame.origin.x + choice.frame.size.width);
                let bottom = (group.frame.origin.y + group.frame.size.height)
                    .max(choice.frame.origin.y + choice.frame.size.height);
                group.frame = tekes_ui_platform_macos::Rect {
                    origin: tekes_ui_platform_macos::Point { x, y },
                    size: tekes_ui_platform_macos::Size {
                        width: right - x,
                        height: bottom - y,
                    },
                };
                choice.parent = Some(group.key);
            }
            nodes.push(group);
            nodes.extend(choices);
        }
        if renderer == "tree" {
            let layout = s.extended.tree_view().layout();
            let model = &s.extended.tree;
            if let Some(frame) = layout.visible {
                let root = K {
                    component: STORY + i as u64,
                    part: Some(29),
                };
                let mut n = node(root, R::Tree, entry(i).title(), frame);
                n.enabled = !s.disabled;
                nodes.push(n);
                let visible = model.visible();
                for item in &layout.items {
                    let Some(frame) = item.visible else {
                        continue;
                    };
                    let Some(row) = model
                        .nodes()
                        .get(item.id)
                        .filter(|_| visible.contains(&item.id))
                    else {
                        continue;
                    };
                    let mut n = node(
                        K {
                            component: root.component,
                            part: Some(30 + item.id),
                        },
                        R::TreeItem { level: item.depth },
                        &row.label,
                        frame,
                    );
                    n.parent = Some(
                        row.parent
                            .filter(|parent| {
                                layout
                                    .items
                                    .iter()
                                    .any(|i| i.id == *parent && i.visible.is_some())
                            })
                            .map_or(root, |parent| K {
                                component: root.component,
                                part: Some(30 + parent),
                            }),
                    );
                    n.enabled = !s.disabled && !row.disabled;
                    n.selected = model.selected == Some(item.id);
                    n.focused = s.window_active && s.focus == Some(30 + item.id);
                    n.expanded = model
                        .nodes()
                        .iter()
                        .any(|row| row.parent == Some(item.id))
                        .then_some(row.expanded);
                    nodes.push(n);
                }
            }
        }
        if renderer == "data-table" {
            let layout = s.extended.table_view().layout();
            let model = &s.extended.table;
            if let Some(frame) = layout.visible {
                let root = K {
                    component: STORY + i as u64,
                    part: Some(29),
                };
                let mut n = node(root, R::Table, entry(i).title(), frame);
                n.enabled = !s.disabled;
                nodes.push(n);
                for header in &layout.headers {
                    let Some(frame) = header.visible.filter(|_| header.id < model.columns) else {
                        continue;
                    };
                    let Some(title) = s.button_labels.get(&(20 + header.id)) else {
                        continue;
                    };
                    let column_key = K {
                        component: root.component,
                        part: Some(100 + header.id),
                    };
                    let column_frame = layout.columns[header.id]
                        .intersection(&layout.visible.unwrap())
                        .unwrap_or(frame);
                    let mut column = node(
                        column_key,
                        R::TableColumn { index: header.id },
                        title,
                        column_frame,
                    );
                    column.parent = Some(root);
                    column.enabled = !s.disabled;
                    nodes.push(column);
                    let sort = model
                        .sort
                        .filter(|(c, _)| *c == header.id)
                        .map(|(_, d)| match d {
                            tekes_ui_controls::extended::SortDirection::Ascending => {
                                tekes_ui_platform_macos::AccessibilitySortDirection::Ascending
                            }
                            tekes_ui_controls::extended::SortDirection::Descending => {
                                tekes_ui_platform_macos::AccessibilitySortDirection::Descending
                            }
                        });
                    let mut n = node(
                        K {
                            component: root.component,
                            part: Some(20 + header.id),
                        },
                        R::TableColumnHeader {
                            index: header.id,
                            sort,
                        },
                        title,
                        frame,
                    );
                    n.parent = Some(column_key);
                    n.enabled = !s.disabled;
                    n.focused = s.window_active && s.focus == Some(20 + header.id);
                    nodes.push(n);
                }
                for (display, row) in layout.rows.iter().enumerate() {
                    let Some(frame) = row.visible.filter(|_| row.id < model.rows) else {
                        continue;
                    };
                    let Some(title) = s.button_labels.get(&(30 + row.id)) else {
                        continue;
                    };
                    let key = K {
                        component: root.component,
                        part: Some(30 + row.id),
                    };
                    let mut n = node(key, R::TableRow { index: display }, title, frame);
                    n.parent = Some(root);
                    n.enabled = !s.disabled;
                    n.selected = model.selected_row == Some(row.id);
                    n.focused = s.window_active && s.focus == Some(30 + row.id);
                    nodes.push(n);
                    for (column, column_frame) in
                        layout.columns.iter().enumerate().take(model.columns)
                    {
                        let Some(cell) = frame.intersection(column_frame) else {
                            continue;
                        };
                        let id = 2000 + row.id * layout.columns.len() + column;
                        let Some(value) = s.button_labels.get(&id) else {
                            continue;
                        };
                        let mut n = node(
                            K {
                                component: root.component,
                                part: Some(id),
                            },
                            R::TableCell {
                                row: display,
                                column,
                            },
                            value,
                            cell,
                        );
                        n.parent = Some(key);
                        n.value = Some(value.clone());
                        n.enabled = !s.disabled;
                        n.read_only = true;
                        nodes.push(n);
                    }
                }
            }
        }
        let popup = match renderer {
            "menu" if s.menu_view.is_open() => s
                .menu_layout
                .as_ref()
                .map(|p| (p, 50, s.menu_view.menu().items.selected)),
            _ => None,
        };
        if let Some((layout, base, active)) = popup
            && let Some(frame) = layout.visible.and_then(|v| layout.popup.intersection(&v))
        {
            for n in &mut nodes {
                n.focused = false;
            }
            let parent = K {
                component: STORY + i as u64,
                part: Some(base - 1),
            };
            let title = s
                .button_labels
                .get(&20)
                .map(String::as_str)
                .unwrap_or(entry(i).title());
            let mut root = node(parent, R::Menu, title, frame);
            let invoker = K {
                component: parent.component,
                part: Some(20),
            };
            root.parent = nodes.iter().any(|n| n.key == invoker).then_some(invoker);
            root.enabled = !s.disabled;
            nodes.push(root);
            for (index, item) in layout.items.iter().enumerate() {
                let Some(frame) = item.visible else {
                    continue;
                };
                let Some(title) = s.button_labels.get(&(base + index)) else {
                    continue;
                };
                let mut n = node(
                    K {
                        component: parent.component,
                        part: Some(base + index),
                    },
                    R::MenuItem,
                    title,
                    frame,
                );
                n.parent = Some(parent);
                n.enabled = !s.disabled;
                n.focused = s.window_active && index == active;
                n.selected = false;
                nodes.push(n);
            }
        }
        if s.logical_focus.iter().any(|t| t.id == 1000 && t.mounted)
            && (renderer != "command-palette" || s.extended.command.combo.open)
        {
            let mut input_nodes = s.input.accessibility_nodes(
                INPUT + i as u64,
                if renderer == "command-palette" {
                    "Command query"
                } else if renderer == "search" {
                    "Search"
                } else {
                    "Project name"
                },
            );
            if renderer == "field-meta" {
                for node in &mut input_nodes {
                    node.required = s.field.required;
                }
            }
            nodes.extend(input_nodes);
        }
        if s.logical_focus.iter().any(|t| t.id == 1001 && t.mounted) {
            nodes.extend(
                s.detail
                    .accessibility_nodes(DETAIL + i as u64, "Document text"),
            );
        }
        if renderer == "command-palette"
            && let Some(frame) = s
                .extended
                .command_view
                .trigger_layout()
                .and_then(|l| l.visible)
        {
            let mut n = node(
                K {
                    component: STORY + i as u64,
                    part: Some(20),
                },
                R::MenuButton,
                "Open command palette",
                frame,
            );
            n.enabled = !s.disabled && !s.extended.command.combo.disabled;
            n.expanded = Some(s.extended.command.combo.open);
            n.focused = s.window_active && s.focus == Some(20);
            nodes.push(n);
        }
        let filtered = match renderer {
            "combo-box" => Some((&s.extended.combo_view.options, &s.extended.combo)),
            "command-palette" => {
                Some((&s.extended.command_view.options, &s.extended.command.combo))
            }
            _ => None,
        };
        if let Some((view, model)) = filtered.filter(|(v, m)| v.is_current(m))
            && let Some(layout) = view.layout()
            && let Some(frame) = layout.visible.and_then(|v| layout.popup.intersection(&v))
        {
            let parent = K {
                component: STORY + i as u64,
                part: Some(39),
            };
            let mut root = node(parent, R::Menu, entry(i).title(), frame);
            let input = K {
                component: INPUT + i as u64,
                part: Some(0),
            };
            root.parent = nodes.iter().any(|n| n.key == input).then_some(input);
            nodes.push(root);
            // Stable semantic keys use source indices, never filtered positions.
            // Only the exact candidates currently painted are actionable.
            for (filtered, item) in layout.items.iter().enumerate() {
                let Some(original) = view.original_index(filtered) else {
                    continue;
                };
                let Some(title) = model.items.get(original) else {
                    continue;
                };
                let Some(frame) = item.visible else {
                    continue;
                };
                if !model.filtered().contains(&original) {
                    continue;
                }
                let mut n = node(
                    K {
                        component: parent.component,
                        part: Some(40 + original),
                    },
                    R::MenuItem,
                    title,
                    frame,
                );
                n.parent = Some(parent);
                n.enabled = !s.disabled;
                n.selected = model.selected == Some(original);
                n.focused = s.window_active && model.active == Some(original);
                nodes.push(n);
            }
            if nodes.iter().any(|n| n.role == R::MenuItem && n.focused) {
                for n in &mut nodes {
                    if n.role != R::MenuItem {
                        n.focused = false;
                    }
                }
            }
        }
        nodes
    }
    pub(in crate::gallery) fn accessibility_action(
        &mut self,
        i: usize,
        key: K,
        action: &A,
        text: &mut super::super::text::TextEngine,
    ) -> bool {
        if !self
            .accessibility_nodes(i)
            .iter()
            .any(|n| n.key == key && n.enabled)
        {
            return false;
        }
        let renderer = entry(i).renderer();
        if self.state(i).input.owns_native_key(key) {
            let s = self.state(i);
            let response = s.input.native_action(key, action.clone());
            let focus_changed = response.consumed
                && matches!(action, A::Focus)
                && s.focus != Some(1000);
            if response.consumed && matches!(action, A::Focus) {
                s.focus = Some(1000);
                s.sync_owned_focus(renderer, s.focus, true);
            }
            if response.changed {
                s.input.refresh(text);
                s.input_changed(renderer);
                for event in s.input.drain_search_logs() {
                    s.record(event);
                }
            }
            return response.changed || focus_changed;
        }
        if renderer == "select" && self.state(i).select_story.owns_native_key(key) {
            let (response, focused, logs) = {
                let state = self.state(i);
                let response = state.select_story.native_action(key, action.clone());
                (response, state.select_story.host.focused().is_some(), state.select_story.drain_logs())
            };
            self.state(i).focus = focused.then_some(20);
            for log in logs { self.state(i).record(log); }
            return response.consumed;
        }
        let Some(id) = key.part else {
            return false;
        };
        if key.component == STORY + i as u64 {
            if matches!(
                (renderer, id),
                ("popover", 20..=21) | ("toast" | "banner" | "empty-state", 20)
            ) {
                if let A::SetExpanded(expanded) = action {
                    if renderer != "popover"
                        || id != 20
                        || self.state(i).extended.popover.open == *expanded
                    {
                        return false;
                    }
                } else if !matches!(action, A::Focus | A::Press) {
                    return false;
                }
                if self.focus_control(i, id).is_none() {
                    return false;
                }
                if matches!(action, A::Focus) {
                    return true;
                }
                let s = self.state(i);
                if renderer == "popover" && id == 20 {
                    s.activate(id, renderer);
                    return true;
                }
                let mut events = Vec::new();
                let down = s
                    .extended
                    .key(renderer, ControlKey::Enter, true, &mut |event| {
                        events.push(event)
                    });
                let up = s
                    .extended
                    .key(renderer, ControlKey::Enter, false, &mut |event| {
                        events.push(event)
                    });
                for event in events {
                    s.record(event);
                }
                if renderer == "popover" {
                    s.focus = s.restore_focus(s.extended.focused);
                    s.sync_owned_focus(renderer, s.focus, true);
                }
                return down || up;
            }
            if renderer == "slider" && id == 20 {
                if !self.state(i).window_active {
                    return false;
                }
                if matches!(action, A::Focus) {
                    return self.focus_control(i, id).is_some();
                }
                let s = self.state(i);
                let model = &mut s.extended.slider;
                let changed = match action {
                    A::Increment => s.extended.slider_view.adjust_value(true, model),
                    A::Decrement => s.extended.slider_view.adjust_value(false, model),
                    A::SetValue(value) => value
                        .parse::<f64>()
                        .ok()
                        .and_then(|v| s.extended.slider_view.set_value(v, model)),
                    _ => None,
                };
                if let Some(changed) = changed {
                    s.extended.cancel_pointer();
                    if changed {
                        s.record(format!("value: {}", s.extended.slider.value()));
                    }
                    return true;
                }
                return false;
            }
            if renderer == "tree"
                && id >= 30
                && let A::SetExpanded(expanded) = action
            {
                let s = self.state(i);
                let event = s.extended.set_tree_expanded(id - 30, *expanded);
                if let tekes_ui_controls::collections_view::CollectionEvent::Toggled(index) = event
                {
                    let visible = s.extended.tree.visible();
                    if s.focus
                        .is_some_and(|id| id >= 30 && !visible.contains(&(id - 30)))
                    {
                        s.focus = Some(30 + index);
                        s.extended.focused = s.focus;
                    }
                    s.record(format!("tree node: {index}"));
                    return true;
                }
                return false;
            }
            if matches!(renderer, "tree" | "data-table") {
                if id == 29 || (renderer == "data-table" && matches!(id, 100..=102 | 2000..)) {
                    return false;
                }
                if matches!(action, A::Focus | A::Press) {
                    let focus_changed = self.state(i).focus != Some(id);
                    if self.focus_control(i, id).is_none() {
                        return false;
                    }
                    if matches!(action, A::Focus) {
                        return focus_changed;
                    }
                    let s = self.state(i);
                    let mut events = Vec::new();
                    let changed = s
                        .extended
                        .activate(renderer, id, &mut |event| events.push(event));
                    for event in events {
                        s.record(event);
                    }
                    return changed || focus_changed;
                }
            }
            if matches!(renderer, "combo-box" | "command-palette") {
                if id == 39 {
                    return false;
                }
                if let Some(original) = id.checked_sub(40).filter(|_| id < 1000) {
                    let s = self.state(i);
                    return match action {
                        A::Focus => {
                            let focused = if renderer == "combo-box" {
                                s.extended
                                    .combo_view
                                    .focus_option(original, &mut s.extended.combo)
                            } else {
                                s.extended
                                    .command_view
                                    .focus_option(original, &mut s.extended.command)
                            };
                            if focused {
                                s.input.focus_field();
                                s.focus = Some(1000);
                                s.sync_owned_focus(renderer, Some(1000), true);
                            }
                            focused
                        }
                        A::Press => {
                            let before = if renderer == "combo-box" {
                                s.extended.combo.open
                            } else {
                                s.extended.command.combo.open
                            };
                            s.activate(id, renderer);
                            let open = if renderer == "combo-box" {
                                s.extended.combo.open
                            } else {
                                s.extended.command.combo.open
                            };
                            if before && !open {
                                s.input.refresh(text);
                                if renderer == "command-palette" {
                                    s.close_command_palette();
                                }
                                return true;
                            }
                            false
                        }
                        _ => false,
                    };
                }
                if renderer == "command-palette" && id == 20 && matches!(action, A::Press) {
                    let s = self.state(i);
                    if s.extended.command.combo.open {
                        s.close_command_palette();
                    } else {
                        s.extended.command_view.open(&mut s.extended.command);
                        s.open_command_palette(text);
                    }
                    return true;
                }
            }
            if self.accessibility_modal(i) {
                if id == 69 {
                    if !matches!(action, A::Focus) {
                        return false;
                    }
                    let s = self.state(i);
                    let index = s.dialog_view.dialog().focus.min(1);
                    let changed = s.dialog_view.focus_button(index);
                    s.focus = Some(70 + index);
                    return changed;
                }
                let Some(index) = id.checked_sub(70).filter(|i| *i < 2) else {
                    return false;
                };
                let s = self.state(i);
                return match action {
                    A::Focus => {
                        let changed = s.dialog_view.focus_button(index);
                        if changed {
                            s.focus = Some(id);
                        }
                        changed
                    }
                    A::Press => {
                        let event = s.dialog_view.activate_button(index);
                        let changed = !matches!(event, DialogEvent::Blocked | DialogEvent::None);
                        s.dialog_event(event, renderer, false);
                        changed
                    }
                    _ => false,
                };
            }
            if matches!((renderer, id), ("menu", 49)) {
                return false; // Published popup containers have no direct action.
            }
            let popup_item = if renderer == "menu" { id.checked_sub(50).filter(|v| *v < 3) } else { None };
            if let Some(index) = popup_item {
                let s = self.state(i);
                return match action {
                    A::Focus => {
                        let focused = s.menu_view.focus_item(index);
                        if focused {
                            s.focus = Some(id);
                        }
                        focused
                    }
                    A::Press => {
                        let event = s.menu_view.activate_item(index);
                        let changed = !matches!(event, MenuEvent::None);
                        s.menu_event(event, renderer, false);
                        changed
                    }
                    _ => false,
                };
            }
            let popup_open = self.states.get(&i).is_some_and(|s| s.menu_view.is_open());
            if popup_open && matches!(action, A::Focus | A::Press) {
                let s = self.state(i);
                let event = s.menu_view.key(ControlKey::Escape, true);
                s.menu_event(event, renderer, false);
                // The expanded trigger dismisses once, just like its outside
                // pointer press; do not close and reopen on the same action.
                if id == 20 && matches!(action, A::Press) {
                    return true;
                }
            }
            match action {
                A::Focus => return self.focus_control(i, id).is_some(),
                A::Press => {
                    if matches!(
                        (renderer, id),
                        ("icon-button" | "link" | "switch", 20)
                            | ("radio-group" | "segmented-control", 30..=32)
                            | ("list", 30..=34)
                    ) {
                        let focus_changed = self.state(i).focus != Some(id);
                        if self.focus_control(i, id).is_none() {
                            return false;
                        }
                        let s = self.state(i);
                        let mut events = Vec::new();
                        let changed = s
                            .extended
                            .activate(renderer, id, &mut |event| events.push(event));
                        for event in events {
                            s.record(event);
                        }
                        return changed || focus_changed;
                    }
                    let s = self.state(i);
                    let Some(button) = s.buttons.get_mut(&id) else {
                        return false;
                    };
                    button.pointer_down();
                    if !button.pointer_up(true) {
                        return false;
                    }
                    s.activate(id, renderer);
                    return true;
                }
                _ => return false,
            }
        }
        let s = self.state(i);
        let detail = key.component == DETAIL + i as u64;
        if !detail && key.component != INPUT + i as u64 {
            return false;
        }
        let before = s.input.query().to_owned();
        let changed = if detail {
            s.detail.accessibility_action(id, action)
        } else {
            s.input.accessibility_action(id, action)
        };
        if detail {
            s.detail.refresh(text);
        } else {
            s.input.refresh(text);
        }
        if changed && matches!(action, A::Focus) {
            s.focus = Some(if detail { 1001 } else { 1000 });
            s.sync_owned_focus(renderer, s.focus, true);
        }
        if s.input.query() != before {
            s.input_changed(renderer);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    fn paint(g: &mut Gallery) {
        g.build_scene(&mut Scene::new());
    }
    fn key(i: usize, part: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(part),
        }
    }
    #[test]
    fn link_exposes_its_own_role_and_owned_activation() {
        let (mut g, i) = gallery("link");
        let nodes = g.accessibility_nodes();
        let link = nodes.iter().find(|n| n.key == key(i, 20)).unwrap();
        assert_eq!(link.role, R::Link);
        assert!(!link.label.is_empty());
        g.workbench.states.get_mut(&i).unwrap().log.clear();
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        assert_eq!(
            g.workbench.states[&i]
                .log
                .iter()
                .filter(|s| s.as_str() == "link: activated")
                .count(),
            1
        );
        assert!(g.accessibility_action(key(i, 10), &A::Press).redraw);
        paint(&mut g);
        assert!(
            !g.accessibility_nodes()
                .iter()
                .find(|n| n.key == key(i, 20))
                .unwrap()
                .enabled
        );
        assert!(!g.accessibility_action(key(i, 20), &A::Press).redraw);
        assert!(!g.accessibility_action(key(i, 20), &A::Focus).redraw);
        assert!(g.accessibility_action(key(i, 10), &A::Press).redraw);
        paint(&mut g);
        g.workbench.set_window_active(false);
        assert!(!g.accessibility_action(key(i, 20), &A::Press).redraw);
        assert_eq!(
            g.workbench.states[&i]
                .log
                .iter()
                .filter(|s| s.as_str() == "link: activated")
                .count(),
            1
        );
    }

    #[test]
    fn feedback_owned_actions_are_real_accessible_children() {
        let mut missing = Vec::new();
        for renderer in ["popover", "toast", "banner", "empty-state"] {
            let (mut g, i) = gallery(renderer);
            if renderer == "popover" {
                assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
                paint(&mut g);
            }
            let id = if renderer == "popover" { 21 } else { 20 };
            let nodes = g.accessibility_nodes();
            if !nodes.iter().any(|n| {
                n.key == key(i, id)
                    && n.role == R::Button
                    && !n.label.is_empty()
                    && n.parent == Some(key(i, 19))
            }) {
                missing.push(renderer);
            }
        }
        assert!(
            missing.is_empty(),
            "component-owned actions missing: {missing:?}"
        );
    }

    #[test]
    fn feedback_accessible_press_reuses_owned_buttons_and_rejects_stale_actions() {
        for renderer in ["popover", "toast", "banner", "empty-state"] {
            let (mut g, i) = gallery(renderer);
            if renderer == "popover" {
                assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
                paint(&mut g);
            }
            let id = if renderer == "popover" { 21 } else { 20 };
            assert!(
                g.accessibility_action(key(i, id), &A::Focus).redraw,
                "{renderer}"
            );
            assert_eq!(g.workbench.states[&i].focus, Some(id));
            assert!(
                g.accessibility_action(key(i, id), &A::Press).redraw,
                "{renderer}"
            );
            let state = &g.workbench.states[&i];
            assert!(
                match renderer {
                    "popover" => !state.extended.popover.open && state.focus == Some(20),
                    "toast" => !state.extended.toast.visible,
                    "banner" => !state.extended.banner.visible,
                    "empty-state" => state.extended.empty_state_created,
                    _ => unreachable!(),
                },
                "{renderer}"
            );
            assert!(
                !g.accessibility_action(key(i, id), &A::Press).redraw,
                "stale {renderer}"
            );
            paint(&mut g);
            assert!(
                !g.accessibility_nodes().iter().any(|n| n.key == key(i, id)),
                "{renderer}"
            );
        }
    }

    #[test]
    fn popover_native_tab_enters_dismiss_instead_of_closing_content() {
        let (mut g, i) = gallery("popover");
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        paint(&mut g);
        assert_eq!(g.workbench.states[&i].focus, Some(20));
        g.handle_input(&Event::TextCommand("insertTab:".into()));
        assert!(g.workbench.states[&i].extended.popover.open);
        assert_eq!(g.workbench.states[&i].focus, Some(21));
        paint(&mut g);
        assert!(
            g.accessibility_nodes()
                .iter()
                .any(|n| n.key == key(i, 21) && n.focused)
        );
        g.handle_input(&Event::TextCommand("insertBacktab:".into()));
        assert!(g.workbench.states[&i].extended.popover.open);
        assert_eq!(g.workbench.states[&i].focus, Some(20));
        g.handle_input(&Event::TextCommand("insertTab:".into()));
        // Non-editable controls receive native key down/up, not text selectors.
        use tekes_ui_platform_macos::{KeyEvent, KeyEventKind, Modifiers};
        for kind in [KeyEventKind::Down, KeyEventKind::Up] {
            g.handle_input(&Event::Key(KeyEvent {
                kind,
                key_code: 36,
                modifiers: Modifiers::NONE,
                is_repeat: false,
                characters: "\r".into(),
                characters_ignoring_modifiers: "\r".into(),
            }));
        }
        assert!(!g.workbench.states[&i].extended.popover.open);
        assert_eq!(g.workbench.states[&i].focus, Some(20));
    }

    #[test]
    fn actual_buttons_export_the_rendered_label_and_ax_press_uses_the_real_activation() {
        let (mut g, i) = gallery("button");
        let key = K {
            component: STORY + i as u64,
            part: Some(20),
        };
        let node = g
            .accessibility_nodes()
            .into_iter()
            .find(|n| n.key == key)
            .expect("actual primary button");
        assert_eq!(node.label, g.workbench.states[&i].button_labels[&20]);
        assert_eq!(node.role, R::Button);
        assert!(node.enabled);
        let before = g.workbench.states[&i].log.len();
        assert!(g.accessibility_action(key, &A::Press).redraw);
        assert_eq!(g.workbench.states[&i].log.len(), before + 1);
        assert!(
            g.workbench.states[&i]
                .log
                .back()
                .unwrap()
                .contains("activate:")
        );
        g.workbench.state(i).disabled = true;
        paint(&mut g);
        assert!(
            !g.accessibility_nodes()
                .iter()
                .find(|n| n.key == key)
                .unwrap()
                .enabled
        );
        let before = g.workbench.states[&i].log.clone();
        assert!(!g.accessibility_action(key, &A::Press).redraw);
        assert_eq!(g.workbench.states[&i].log, before);
    }
    #[test]
    fn actual_text_editor_ax_edits_selection_and_native_input_state_stay_in_sync() {
        let (mut g, i) = gallery("field");
        let key = K {
            component: INPUT + i as u64,
            part: Some(0),
        };
        assert!(g.accessibility_nodes().iter().any(|n| n.key == key));
        g.accessibility_action(key, &A::Focus);
        assert!(
            g.accessibility_action(key, &A::SetValue("中文 😀 e\u{301}".into()))
                .redraw
        );
        paint(&mut g);
        let n = g
            .accessibility_nodes()
            .into_iter()
            .find(|n| n.key == key)
            .unwrap();
        assert_eq!(n.value.as_deref(), Some("中文 😀 e\u{301}"));
        assert_eq!(n.text_selection, Some(8..8));
        assert!(g.accessibility_action(key, &A::SetSelection(3..5)).redraw);
        assert!(
            g.accessibility_action(key, &A::ReplaceSelectedText("🙂".into()))
                .redraw
        );
        assert_eq!(g.workbench.states[&i].input.query(), "中文 🙂 e\u{301}");
        let state = g.text_input_state().unwrap();
        assert_eq!(state.text, "中文 🙂 e\u{301}");
        assert_eq!(state.selection, 5..5);
    }
    #[test]
    fn actual_route_and_story_changes_remove_old_nodes_and_keys_cannot_activate_them() {
        let (mut g, i) = gallery("field");
        let key = K {
            component: INPUT + i as u64,
            part: Some(0),
        };
        assert!(g.accessibility_nodes().iter().any(|n| n.key == key));
        let docs = K {
            component: STORY + i as u64,
            part: Some(1),
        };
        assert!(g.accessibility_action(docs, &A::Press).redraw);
        paint(&mut g);
        assert!(!g.accessibility_nodes().iter().any(|n| n.key == key));
        assert!(
            !g.accessibility_action(key, &A::SetValue("late".into()))
                .redraw
        );
        let j = entries()
            .iter()
            .position(|e| e.renderer() == "button")
            .unwrap();
        g.apply(crate::gallery::Command::Page(Page::Workbench(j)));
        paint(&mut g);
        assert!(!g.accessibility_action(docs, &A::Press).redraw);
    }
}

#[cfg(test)]
mod modal_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    #[test]
    fn real_dialog_exports_only_its_scope_and_native_press_restores_the_caller_once() {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "dialog")
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        // Open through the actual AX path; the mounted story container keeps
        // its own identity even before the next layout creates dialog children.
        let page_key = K {
            component: STORY + i as u64,
            part: None,
        };
        assert!(
            g.accessibility_nodes()
                .iter()
                .any(|n| n.key == page_key && n.role == R::Group)
        );
        assert!(
            g.accessibility_action(
                K {
                    component: STORY + i as u64,
                    part: Some(20)
                },
                &A::Press
            )
            .redraw
        );
        assert!(
            g.accessibility_nodes()
                .iter()
                .any(|n| n.key == page_key && n.role == R::Group)
        );
        g.build_scene(&mut Scene::new());
        assert!(g.workbench.accessibility_modal(i));
        let nodes = g.accessibility_nodes();
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.key.component == STORY + i as u64 && n.key.part.is_some())
                .count(),
            3
        );
        assert!(
            !nodes
                .iter()
                .any(|n| n.key.component == STORY + i as u64 && n.key.part.is_some_and(|p| p < 69))
        );
        let dialog = nodes.iter().find(|n| n.role == R::Dialog).unwrap();
        assert!(
            nodes
                .iter()
                .filter(|n| n.role == R::Button && n.key.component == dialog.key.component)
                .all(|n| n.parent == Some(dialog.key))
        );
        let cancel = nodes.iter().find(|n| n.label == "Cancel").unwrap().key;
        // The sidebar is outside this preview's modal scope, just as in the
        // existing Gallery pointer/keyboard host. Story underlay stays hidden.
        assert!(
            nodes
                .iter()
                .any(|n| n.key.component == crate::gallery::accessibility::NAV)
        );
        assert!(g.accessibility_action(cancel, &A::Press).redraw);
        assert!(!g.workbench.accessibility_modal(i));
        assert_eq!(g.workbench.states[&i].focus, Some(20));
        assert!(!g.accessibility_action(cancel, &A::Press).redraw);
        assert!(!g.accessibility_nodes().iter().any(|n| n.role == R::Dialog));
    }
}

#[cfg(test)]
mod isolation_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    #[test]
    fn published_parent_links_and_keys_are_complete_and_clipped_to_actual_slots() {
        for renderer in ["button", "field", "area", "search", "dialog"] {
            let (g, _) = gallery(renderer);
            let nodes = g.accessibility_nodes();
            let keys: std::collections::HashSet<_> = nodes.iter().map(|n| n.key).collect();
            assert_eq!(keys.len(), nodes.len());
            for n in &nodes {
                if let Some(parent) = n.parent {
                    assert!(keys.contains(&parent));
                }
                assert!(n.frame.size.width > 0. && n.frame.size.height > 0.);
            }
            if renderer == "area" {
                assert!(nodes.iter().any(|n| n.role == R::TextArea));
            }
            if renderer == "field" {
                assert!(nodes.iter().any(|n| n.role == R::TextField));
            }
        }
    }
    #[test]
    fn composing_editor_rejects_ax_navigation_value_and_selection_without_disrupting_preedit() {
        let (mut g, i) = gallery("field");
        let key = K {
            component: INPUT + i as u64,
            part: Some(0),
        };
        g.accessibility_action(key, &A::Focus);
        g.handle_input(&Event::TextPreedit {
            text: "拼".into(),
            selection: 0..1,
            clauses: Vec::new(),
            replacement: None,
        });
        let before = g.text_input_state().unwrap();
        assert!(before.marked.is_some());
        assert!(
            !g.accessibility_action(key, &A::SetValue("stale".into()))
                .redraw
        );
        assert!(!g.accessibility_action(key, &A::SetSelection(0..0)).redraw);
        let nav = g
            .accessibility_nodes()
            .iter()
            .find(|n| n.key.component == crate::gallery::accessibility::NAV && n.key.part.is_some())
            .unwrap()
            .key;
        assert!(!g.accessibility_action(nav, &A::Press).redraw);
        let after = g.text_input_state().unwrap();
        assert_eq!(before.text, after.text);
        assert_eq!(before.marked, after.marked);
        assert_eq!(before.selection, after.selection);
    }
}

#[cfg(test)]
mod text_family_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    #[test]
    fn all_three_actual_text_variants_share_ax_editing_and_expose_the_correct_native_role() {
        for renderer in ["field", "area", "search"] {
            let i = entries()
                .iter()
                .position(|e| e.renderer() == renderer)
                .unwrap();
            let mut g = Gallery::with_static_text(StaticText::new(None));
            g.page = Page::Workbench(i);
            g.set_view_size(1200., 850.);
            g.build_scene(&mut Scene::new());
            let key = if renderer == "search" {
                g.accessibility_nodes()
                    .into_iter()
                    .find(|node| node.key.component >= (1_u64 << 63) && node.value.is_some())
                    .unwrap()
                    .key
            } else {
                K {
                    component: INPUT + i as u64,
                    part: Some(0),
                }
            };
            g.accessibility_action(key, &A::Focus);
            assert!(
                g.accessibility_action(key, &A::SetValue("中文 😀 e\u{301}".into()))
                    .redraw
            );
            g.build_scene(&mut Scene::new());
            let n = g
                .accessibility_nodes()
                .into_iter()
                .find(|n| n.key == key)
                .unwrap();
            assert_eq!(
                n.role,
                match renderer {
                    "area" => R::TextArea,
                    "search" => R::SearchField,
                    _ => R::TextField,
                }
            );
            assert_eq!(n.value.as_deref(), Some("中文 😀 e\u{301}"));
            assert!(g.accessibility_action(key, &A::SetSelection(3..5)).redraw);
            assert!(
                g.accessibility_action(key, &A::ReplaceSelectedText("🙂".into()))
                    .redraw
            );
            assert_eq!(g.text_input_state().unwrap().text, "中文 🙂 e\u{301}");
            g.workbench.state(i).error = true;
            g.build_scene(&mut Scene::new());
            assert!(
                g.accessibility_nodes()
                    .iter()
                    .find(|n| n.key == key)
                    .unwrap()
                    .invalid
            );
        }
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    fn key(i: usize, id: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(id),
        }
    }
    fn node_at(g: &Gallery, i: usize, id: usize) -> N {
        let matches: Vec<_> = g
            .accessibility_nodes()
            .into_iter()
            .filter(|n| n.key == key(i, id))
            .collect();
        assert_eq!(matches.len(), 1, "one actual mounted owner for {id}");
        matches[0].clone()
    }
    #[test]
    fn actual_slider_exports_range_and_semantic_edits_use_original_model_and_mount() {
        let (mut g, i) = gallery("slider");
        let n = node_at(&g, i, 20);
        assert!(
            matches!(n.role, R::Slider(range) if range.minimum() == 0. && range.maximum() == 100.)
        );
        assert_eq!(n.value.as_deref(), Some("50"));
        assert!(!g.accessibility_action(key(i, 20), &A::Press).redraw);
        assert!(g.accessibility_action(key(i, 20), &A::Increment).redraw);
        assert_eq!(g.workbench.states[&i].extended.slider.value(), 55.);
        assert!(
            g.accessibility_action(key(i, 20), &A::SetValue("63".into()))
                .redraw
        );
        assert_eq!(g.workbench.states[&i].extended.slider.value(), 65.);
        assert!(
            !g.accessibility_action(key(i, 20), &A::SetValue("NaN".into()))
                .redraw
        );
        g.workbench
            .state(i)
            .set_window_active(false, std::time::Duration::ZERO);
        assert!(!g.accessibility_action(key(i, 20), &A::Decrement).redraw);
        assert_eq!(g.workbench.states[&i].extended.slider.value(), 65.);
    }
    #[test]
    fn actual_slider_boundary_requests_are_consumed_and_disabled_or_stale_targets_rejected() {
        let (mut g, i) = gallery("slider");
        assert!(g.accessibility_action(key(i, 20), &A::Focus).redraw);
        assert!(
            g.accessibility_action(key(i, 20), &A::SetValue("999".into()))
                .redraw
        );
        assert_eq!(g.workbench.states[&i].extended.slider.value(), 100.);
        assert!(g.accessibility_action(key(i, 20), &A::Increment).redraw);
        assert_eq!(g.workbench.states[&i].extended.slider.value(), 100.);
        g.workbench.state(i).disabled = true;
        g.build_scene(&mut Scene::new());
        assert!(!node_at(&g, i, 20).enabled);
        assert!(!g.accessibility_action(key(i, 20), &A::Decrement).redraw);
        let other = entries()
            .iter()
            .position(|e| e.renderer() == "button")
            .unwrap();
        g.apply(crate::gallery::Command::Page(Page::Workbench(other)));
        assert!(!g.accessibility_action(key(i, 20), &A::Focus).redraw);
    }
    #[test]
    fn actual_slider_does_not_reinterpret_return_or_space_as_button_activation() {
        use tekes_ui_platform_macos::{Event, KeyEvent, KeyEventKind, Modifiers};
        let (mut g, i) = gallery("slider");
        assert!(g.accessibility_action(key(i, 20), &A::Focus).redraw);
        assert_eq!(g.workbench.states[&i].focus, Some(20));
        for key_code in [36, 49] {
            for kind in [KeyEventKind::Down, KeyEventKind::Up] {
                g.workbench.handle(
                    i,
                    &Event::Key(KeyEvent {
                        kind,
                        key_code,
                        modifiers: Modifiers::NONE,
                        is_repeat: false,
                        characters: String::new(),
                        characters_ignoring_modifiers: String::new(),
                    }),
                    &mut g.text,
                );
                assert_eq!(g.workbench.states[&i].extended.slider.value(), 50.);
            }
        }
    }
    #[test]
    fn real_icon_and_switch_publish_original_name_state_and_use_original_actions() {
        for renderer in ["icon-button", "switch"] {
            let (mut g, i) = gallery(renderer);
            let before = node_at(&g, i, 20);
            assert_eq!(
                before.role,
                if renderer == "switch" {
                    R::Switch
                } else {
                    R::Button
                }
            );
            assert!(!before.label.is_empty());
            if renderer == "icon-button" {
                assert_eq!(
                    before.label,
                    g.workbench.states[&i].extended.icon_button_view.name()
                );
            }
            let log = g.workbench.states[&i].log.len();
            assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert_eq!(g.workbench.states[&i].log.len(), log + 1);
            let after = node_at(&g, i, 20);
            assert!(after.focused);
            if renderer == "switch" {
                assert_ne!(before.selected, after.selected);
                assert_ne!(before.label, after.label);
            }
            assert!(g.accessibility_action(key(i, 10), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert!(!node_at(&g, i, 20).enabled);
            assert!(!g.accessibility_action(key(i, 20), &A::Press).redraw);
            assert!(!g.accessibility_action(key(i, 20), &A::Focus).redraw);
        }
    }
    #[test]
    fn actual_radio_and_segments_preserve_exclusive_selection_parents_and_roving_focus() {
        for renderer in ["radio-group", "segmented-control"] {
            let (mut g, i) = gallery(renderer);
            assert_eq!(node_at(&g, i, 29).role, R::RadioGroup);
            for id in 30..33 {
                let n = node_at(&g, i, id);
                assert_eq!(n.role, R::RadioButton);
                assert_eq!(n.parent, Some(key(i, 29)));
                assert_eq!(n.label, g.workbench.states[&i].button_labels[&id]);
            }
            let initial = (30..33).find(|id| node_at(&g, i, *id).selected);
            if let Some(id) = initial {
                let log = g.workbench.states[&i].log.len();
                assert!(
                    g.accessibility_action(key(i, id), &A::Press).redraw,
                    "unchanged selection can still acquire focus"
                );
                g.build_scene(&mut Scene::new());
                assert!(node_at(&g, i, id).focused);
                assert_eq!(
                    g.workbench.states[&i].log.len(),
                    log,
                    "no fabricated selection change"
                );
            }
            let target = if initial == Some(31) { 32 } else { 31 };
            assert!(g.accessibility_action(key(i, target), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert!(node_at(&g, i, target).selected);
            assert!(node_at(&g, i, target).focused);
            assert_eq!(
                (30..33).filter(|id| node_at(&g, i, *id).selected).count(),
                1
            );
            assert!(
                !g.accessibility_action(key(i, target), &A::Press).redraw,
                "selected item is an existing no-op"
            );
            assert!(g.accessibility_action(key(i, 10), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            for id in 30..33 {
                assert!(!node_at(&g, i, id).enabled);
                assert!(!g.accessibility_action(key(i, id), &A::Press).redraw);
            }
        }
    }
    #[test]
    fn real_checkbox_preserves_mixed_value_and_normal_toggle() {
        let (mut g, i) = gallery("checkbox");
        g.workbench.state(i).checkbox.checked = CheckState::Mixed;
        g.build_scene(&mut Scene::new());
        let n = node_at(&g, i, 20);
        assert_eq!(n.role, R::Checkbox);
        assert_eq!(n.value.as_deref(), Some("mixed"));
        assert!(!n.selected);
        assert_eq!(n.label, g.workbench.states[&i].button_labels[&20]);
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        g.build_scene(&mut Scene::new());
        assert_eq!(
            node_at(&g, i, 20).selected,
            g.workbench.states[&i].checkbox.checked == CheckState::Checked
        );
        assert_ne!(node_at(&g, i, 20).value.as_deref(), Some("mixed"));
    }
}

#[cfg(test)]
mod list_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn gallery() -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "list")
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    fn key(i: usize, id: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(id),
        }
    }
    #[test]
    fn actual_list_exports_original_rows_and_repeated_ax_press_is_an_activation() {
        let (mut g, i) = gallery();
        let nodes = g.accessibility_nodes();
        let list = nodes.iter().find(|n| n.key == key(i, 29)).unwrap();
        assert_eq!(list.role, R::List);
        let rows: Vec<_> = nodes
            .iter()
            .filter(|n| n.role == R::ListItem && n.parent == Some(list.key))
            .collect();
        assert_eq!(rows.len(), 5);
        for (id, label) in ["Overview", "Documents", "Images", "Archive", "Trash"]
            .iter()
            .enumerate()
        {
            let n = rows.iter().find(|n| n.key == key(i, 30 + id)).unwrap();
            assert_eq!(&n.label, label);
            assert_eq!(n.label, g.workbench.states[&i].button_labels[&(30 + id)]);
        }
        assert!(g.accessibility_action(key(i, 32), &A::Focus).redraw);
        assert_eq!(
            g.workbench.states[&i].extended.list.selected, None,
            "focus does not select"
        );
        for _ in 0..2 {
            let before = g.workbench.states[&i].log.len();
            assert!(g.accessibility_action(key(i, 32), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert_eq!(g.workbench.states[&i].extended.list.selected, Some(2));
            assert_eq!(g.workbench.states[&i].log.len(), before + 1);
            assert_eq!(
                g.workbench.states[&i].log.back().unwrap(),
                "list: activated 2"
            );
            let nodes = g.accessibility_nodes();
            let rows: Vec<_> = nodes.iter().filter(|n| n.role == R::ListItem).collect();
            assert_eq!(rows.iter().filter(|n| n.selected).count(), 1);
            assert!(rows.iter().find(|n| n.key == key(i, 32)).unwrap().focused);
        }
        assert!(g.accessibility_action(key(i, 1), &A::Press).redraw);
        g.build_scene(&mut Scene::new());
        assert!(
            !g.accessibility_nodes()
                .iter()
                .any(|n| n.role == R::ListItem)
        );
        assert!(!g.accessibility_action(key(i, 32), &A::Press).redraw);
    }
    #[test]
    fn collection_activation_success_uses_consumption_even_without_a_selection_delta() {
        for renderer in ["list", "tree", "data-table"] {
            let i = entries()
                .iter()
                .position(|e| e.renderer() == renderer)
                .unwrap();
            let mut g = Gallery::with_static_text(StaticText::new(None));
            g.page = Page::Workbench(i);
            g.set_view_size(1200., 850.);
            g.build_scene(&mut Scene::new());
            let target = g.workbench.states[&i]
                .logical_focus
                .iter()
                .find(|t| t.id >= 30 && t.mounted && t.eligible)
                .unwrap()
                .id;
            assert!(g.workbench.focus_control(i, target).is_some());
            let s = g.workbench.state(i);
            let mut log = Vec::new();
            for _ in 0..2 {
                assert!(
                    s.extended
                        .activate(renderer, target, &mut |event| log.push(event)),
                    "{renderer}: retained Enter activation must succeed twice"
                );
            }
            assert!(
                log.iter().any(|event| event.contains("activated")),
                "original collection activation event: {renderer}"
            );
        }
    }
    #[test]
    fn actual_list_hidden_rows_are_not_ax_targets_and_disabled_rows_cannot_activate() {
        let (mut g, i) = gallery();
        let mut partial = false;
        for height in (360..700).step_by(20) {
            g.set_view_size(1200., height as f32);
            g.build_scene(&mut Scene::new());
            let rows = g
                .accessibility_nodes()
                .iter()
                .filter(|n| n.role == R::ListItem)
                .count();
            if rows > 0
                && rows < 5
                && g.workbench.states[&i]
                    .logical_focus
                    .iter()
                    .any(|t| t.id == 34 && t.mounted)
            {
                partial = true;
                assert!(!g.accessibility_nodes().iter().any(|n| n.key == key(i, 34)));
                assert!(!g.accessibility_action(key(i, 34), &A::Press).redraw);
                break;
            }
        }
        assert!(
            partial,
            "real bounded viewport with a logically mounted but clipped row"
        );
        g.set_view_size(1200., 850.);
        g.workbench.state(i).extended.disabled = true;
        g.build_scene(&mut Scene::new());
        for id in 30..35 {
            assert!(
                !g.accessibility_nodes()
                    .iter()
                    .find(|n| n.key == key(i, id))
                    .unwrap()
                    .enabled
            );
            assert!(!g.accessibility_action(key(i, id), &A::Press).redraw);
        }
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    fn key(i: usize, id: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(id),
        }
    }
    #[test]
    fn real_menu_and_select_export_popup_roles_and_activate_original_models_once() {
        for renderer in ["menu"] {
            let (mut g, i) = gallery(renderer);
            let base = if renderer == "menu" { 50 } else { 60 };
            let trigger = g
                .accessibility_nodes()
                .into_iter()
                .find(|n| n.key == key(i, 20))
                .unwrap();
            assert_eq!(trigger.role, R::MenuButton);
            assert_eq!(trigger.expanded, Some(false));
            assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            let nodes = g.accessibility_nodes();
            let root = nodes.iter().find(|n| n.role == R::Menu).unwrap();
            assert_eq!(root.key, key(i, base - 1));
            assert_eq!(root.parent, Some(key(i, 20)));
            assert!(!g.accessibility_action(key(i, base - 1), &A::Press).redraw);
            assert!(!g.accessibility_action(key(i, base - 1), &A::Focus).redraw);
            let nodes = g.accessibility_nodes();
            assert!(nodes.iter().any(|n| n.role == R::Menu));
            let items: Vec<_> = nodes.iter().filter(|n| n.role == R::MenuItem).collect();
            assert_eq!(items.len(), 3);
            assert!(items.iter().all(|n| n.parent == Some(root.key)));
            for n in &items {
                assert_eq!(
                    n.label,
                    g.workbench.states[&i].button_labels[&n.key.part.unwrap()]
                );
            }
            assert_eq!(nodes.iter().filter(|n| n.focused).count(), 1);
            let before = g.workbench.states[&i].select_story.model().selected;
            assert!(g.accessibility_action(key(i, base + 2), &A::Focus).redraw);
            g.build_scene(&mut Scene::new());
            if renderer == "select" {
                assert_eq!(g.workbench.states[&i].select_story.model().selected, before);
            }
            assert!(
                g.accessibility_nodes()
                    .iter()
                    .find(|n| n.key == key(i, base + 2))
                    .unwrap()
                    .focused
            );
            let log = g.workbench.states[&i].log.len();
            assert!(g.accessibility_action(key(i, base + 2), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert_eq!(g.workbench.states[&i].log.len(), log + 1);
            assert_eq!(g.workbench.states[&i].focus, Some(20));
            assert!(
                !g.accessibility_nodes()
                    .iter()
                    .any(|n| n.role == R::MenuItem || n.role == R::Menu)
            );
            assert!(!g.accessibility_action(key(i, base + 2), &A::Press).redraw);
            if renderer == "select" {
                assert_eq!(g.workbench.states[&i].select_story.model().selected, 2);
                assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
                g.build_scene(&mut Scene::new());
                let selected = g
                    .accessibility_nodes()
                    .into_iter()
                    .find(|n| n.key == key(i, base + 2))
                    .unwrap();
                assert!(
                    selected.focused && selected.selected,
                    "reopen must focus the current committed option"
                );
            } else {
                assert_eq!(g.workbench.states[&i].menu_view.menu().last_action, Some(2));
            }
        }
    }
    #[test]
    fn real_expanded_trigger_dismisses_without_reopening_and_background_focus_uses_escape() {
        for renderer in ["menu"] {
            let (mut g, i) = gallery(renderer);
            for background in [false, true] {
                assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
                g.build_scene(&mut Scene::new());
                assert!(g.accessibility_nodes().iter().any(|n| n.role == R::Menu));
                assert!(
                    g.accessibility_action(
                        key(i, if background { 1 } else { 20 }),
                        if background { &A::Focus } else { &A::Press }
                    )
                    .redraw
                );
                g.build_scene(&mut Scene::new());
                assert!(!g.accessibility_nodes().iter().any(|n| n.role == R::Menu));
                assert_eq!(
                    g.accessibility_nodes()
                        .iter()
                        .find(|n| n.key == key(i, 20))
                        .unwrap()
                        .expanded,
                    Some(false)
                );
            }
        }
    }
    #[test]
    fn outer_gallery_focus_transfer_dismisses_menu_and_select_without_restoring_underlay() {
        for renderer in ["menu"] {
            for search in [true, false] {
                let (mut g, i) = gallery(renderer);
                assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
                g.build_scene(&mut Scene::new());
                let target = if search {
                    K {
                        component: crate::gallery::accessibility::SEARCH,
                        part: Some(0),
                    }
                } else {
                    g.accessibility_nodes()
                        .iter()
                        .find(|n| {
                            n.key.component == crate::gallery::accessibility::NAV
                                && n.key.part.is_some()
                        })
                        .unwrap()
                        .key
                };
                assert!(g.accessibility_action(target, &A::Focus).redraw);
                g.build_scene(&mut Scene::new());
                let nodes = g.accessibility_nodes();
                assert!(
                    !nodes
                        .iter()
                        .any(|n| n.role == R::Menu || n.role == R::MenuItem),
                    "{renderer}: outer focus must leave no interactive popup"
                );
                assert_eq!(nodes.iter().filter(|n| n.focused).count(), 1);
                assert!(nodes.iter().find(|n| n.key == target).unwrap().focused);
            }
        }
    }
    #[test]
    fn disabled_menu_popup_items_reject_semantic_activation() {
        let (mut g, i) = gallery("menu");
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        g.workbench.state(i).disabled = true;
        g.build_scene(&mut Scene::new());
        assert!(!g.accessibility_action(key(i, 50), &A::Press).redraw);
    }
}

#[cfg(test)]
mod filtered_menu_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn key(i: usize, id: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(id),
        }
    }
    fn input(i: usize) -> K {
        K {
            component: INPUT + i as u64,
            part: Some(0),
        }
    }
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        if renderer == "command-palette" {
            assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
        }
        (g, i)
    }
    #[test]
    fn filtered_real_consumers_export_source_indices_focus_without_commit_and_confirm_once() {
        for (renderer, query, expected) in [
            ("combo-box", "Com", "Comfortable"),
            ("command-palette", "Search", "Search files"),
        ] {
            let (mut g, i) = gallery(renderer);
            g.accessibility_action(input(i), &A::Focus);
            assert!(
                g.accessibility_action(input(i), &A::SetValue(query.into()))
                    .redraw
            );
            g.build_scene(&mut Scene::new());
            let nodes = g.accessibility_nodes();
            assert!(!nodes.iter().any(|n| n.key == key(i, 40)));
            let candidate = nodes.iter().find(|n| n.key == key(i, 42)).unwrap();
            assert_eq!(candidate.label, expected);
            assert_eq!(candidate.role, R::MenuItem);
            assert_eq!(candidate.parent, Some(key(i, 39)));
            assert!(!g.accessibility_action(key(i, 39), &A::Press).redraw);
            assert!(g.accessibility_action(key(i, 42), &A::Focus).redraw);
            let state = &g.workbench.states[&i];
            let model = if renderer == "combo-box" {
                &state.extended.combo
            } else {
                &state.extended.command.combo
            };
            assert_eq!(model.active, Some(2));
            assert_eq!(model.selected, None);
            assert!(g.accessibility_action(key(i, 42), &A::Press).redraw);
            let state = &g.workbench.states[&i];
            assert_eq!(state.input.query(), expected);
            assert_eq!(
                state
                    .log
                    .iter()
                    .filter(|s| s.starts_with(if renderer == "combo-box" {
                        "combo selection:"
                    } else {
                        "command:"
                    }))
                    .count(),
                1
            );
            g.build_scene(&mut Scene::new());
            assert!(!g.accessibility_nodes().iter().any(|n| n.key == key(i, 42)));
            assert!(!g.accessibility_action(key(i, 42), &A::Press).redraw);
            if renderer == "command-palette" {
                assert_eq!(g.workbench.states[&i].focus, Some(20));
            }
        }
    }
    #[test]
    fn stale_candidates_empty_query_and_preedit_do_not_retarget_or_commit() {
        for renderer in ["combo-box", "command-palette"] {
            let (mut g, i) = gallery(renderer);
            g.accessibility_action(input(i), &A::Focus);
            g.accessibility_action(input(i), &A::SetValue("a".into()));
            g.build_scene(&mut Scene::new());
            g.accessibility_action(input(i), &A::SetValue("no matching candidate".into()));
            assert!(!g.accessibility_action(key(i, 42), &A::Press).redraw);
            g.build_scene(&mut Scene::new());
            assert!(!g.accessibility_nodes().iter().any(|n| n.key == key(i, 39)));
            g.accessibility_action(input(i), &A::SetValue("".into()));
            g.build_scene(&mut Scene::new());
            g.handle_input(&Event::TextPreedit {
                text: "拼".into(),
                selection: 0..1,
                clauses: Vec::new(),
                replacement: None,
            });
            let before = g.text_input_state().unwrap();
            assert!(before.marked.is_some());
            assert!(!g.accessibility_action(key(i, 42), &A::Press).redraw);
            assert_eq!(g.text_input_state().unwrap().marked, before.marked);
        }
    }
    #[test]
    fn outer_focus_dismisses_both_filtered_consumers() {
        for renderer in ["combo-box", "command-palette"] {
            let (mut g, i) = gallery(renderer);
            g.accessibility_action(input(i), &A::Focus);
            g.accessibility_action(input(i), &A::SetValue("a".into()));
            g.build_scene(&mut Scene::new());
            let search = K {
                component: crate::gallery::accessibility::SEARCH,
                part: Some(0),
            };
            assert!(g.accessibility_action(search, &A::Focus).redraw);
            g.build_scene(&mut Scene::new());
            assert!(!g.accessibility_nodes().iter().any(|n| n.key == key(i, 39)));
        }
    }
}

#[cfg(test)]
mod tree_table_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};
    fn key(i: usize, id: usize) -> K {
        K {
            component: STORY + i as u64,
            part: Some(id),
        }
    }
    fn gallery(renderer: &str) -> (Gallery, usize) {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        (g, i)
    }
    #[test]
    fn actual_tree_uses_branch_relationships_explicit_disclosure_and_original_activation() {
        let (mut g, i) = gallery("tree");
        let nodes = g.accessibility_nodes();
        assert_eq!(
            nodes.iter().find(|n| n.key == key(i, 29)).unwrap().role,
            R::Tree
        );
        let child = nodes.iter().find(|n| n.key == key(i, 31)).unwrap();
        assert_eq!(child.parent, Some(key(i, 30)));
        assert_eq!(child.role, R::TreeItem { level: 1 });
        assert!(g.accessibility_action(key(i, 31), &A::Press).redraw);
        assert_eq!(g.workbench.states[&i].extended.tree.selected, Some(1));
        assert!(
            g.accessibility_action(key(i, 30), &A::SetExpanded(false))
                .redraw
        );
        assert_eq!(g.workbench.states[&i].extended.tree.selected, Some(0));
        assert_eq!(g.workbench.states[&i].focus, Some(30));
        assert!(!g.accessibility_nodes().iter().any(|n| n.key == key(i, 31)));
        assert!(!g.accessibility_action(key(i, 31), &A::Press).redraw);
        assert!(
            !g.accessibility_action(key(i, 30), &A::SetExpanded(false))
                .redraw
        );
        g.build_scene(&mut Scene::new());
        assert!(
            g.accessibility_action(key(i, 30), &A::SetExpanded(true))
                .redraw
        );
        g.build_scene(&mut Scene::new());
        assert!(g.accessibility_nodes().iter().any(|n| n.key == key(i, 31)));
        assert!(
            !g.accessibility_action(key(i, 31), &A::SetExpanded(true))
                .redraw
        );
        let before = g.workbench.states[&i].log.len();
        assert!(g.accessibility_action(key(i, 30), &A::Press).redraw);
        assert_eq!(g.workbench.states[&i].log.len(), before + 1);
        assert_eq!(
            g.workbench.states[&i].log.back().unwrap(),
            "tree: activated 0"
        );
    }
    #[test]
    fn actual_table_exposes_cells_columns_sort_and_stable_source_selection() {
        let (mut g, i) = gallery("data-table");
        let nodes = g.accessibility_nodes();
        assert_eq!(
            nodes.iter().find(|n| n.key == key(i, 29)).unwrap().role,
            R::Table
        );
        assert_eq!(
            nodes
                .iter()
                .filter(|n| matches!(n.role, R::TableColumn { .. }))
                .count(),
            3
        );
        assert_eq!(
            nodes
                .iter()
                .filter(|n| matches!(n.role, R::TableCell { .. }))
                .count(),
            15
        );
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        g.build_scene(&mut Scene::new());
        let nodes = g.accessibility_nodes();
        assert_eq!(
            nodes.iter().find(|n| n.key == key(i, 20)).unwrap().role,
            R::TableColumnHeader {
                index: 0,
                sort: Some(tekes_ui_platform_macos::AccessibilitySortDirection::Ascending)
            }
        );
        let first = g.workbench.states[&i].extended.visible_rows[0];
        assert_eq!(first, 3);
        assert_eq!(
            nodes.iter().find(|n| n.key == key(i, 33)).unwrap().role,
            R::TableRow { index: 0 }
        );
        assert_eq!(
            nodes
                .iter()
                .find(|n| n.key == key(i, 2009))
                .unwrap()
                .value
                .as_deref(),
            Some("Archive 4")
        );
        assert!(g.accessibility_action(key(i, 33), &A::Press).redraw);
        assert_eq!(g.workbench.states[&i].extended.table.selected_row, Some(3));
        assert!(!g.accessibility_action(key(i, 2009), &A::Press).redraw);
        assert!(!g.accessibility_action(key(i, 100), &A::Press).redraw);
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
        g.build_scene(&mut Scene::new());
        assert_eq!(
            g.workbench.states[&i].extended.table.sort,
            Some((0, tekes_ui_controls::extended::SortDirection::Descending))
        );
    }
    #[test]
    fn table_column_shrink_before_repaint_keeps_snapshot_cell_identity() {
        let (mut g, i) = gallery("data-table");
        g.workbench.state(i).extended.table.columns = 1;
        let nodes = g.accessibility_nodes();
        let cell = nodes.iter().find(|n| n.key == key(i, 2003)).unwrap();
        assert_eq!(cell.value.as_deref(), Some("Notes 2"));
        assert_eq!(cell.role, R::TableCell { row: 1, column: 0 });
        assert!(!nodes.iter().any(|n| n.key == key(i, 21)));
        assert!(!g.accessibility_action(key(i, 21), &A::Press).redraw);
    }
    #[test]
    fn newly_mounted_and_reset_collections_inherit_inactive_window_state() {
        let (mut g, initial) = gallery("tree");
        g.handle_input(&Event::Focus(false));
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "data-table")
            .unwrap();
        assert_ne!(i, initial);
        assert!(!g.workbench.states.contains_key(&i));
        g.page = Page::Workbench(i);
        g.build_scene(&mut Scene::new());
        assert!(!g.workbench.states[&i].window_active);
        assert!(!g.accessibility_action(key(i, 30), &A::Focus).redraw);
        assert!(!g.accessibility_action(key(i, 20), &A::Press).redraw);
        g.workbench.state(i).activate(0, "data-table");
        assert!(!g.workbench.states[&i].window_active);
        g.handle_input(&Event::Focus(true));
        g.build_scene(&mut Scene::new());
        assert!(g.accessibility_action(key(i, 20), &A::Press).redraw);
    }
    #[test]
    fn real_collection_lifecycle_rejects_inactive_disabled_and_unmounted_targets() {
        for renderer in ["tree", "data-table"] {
            let (mut g, i) = gallery(renderer);
            g.handle_input(&Event::Focus(false));
            let before = g.workbench.states[&i].focus;
            assert!(!g.accessibility_action(key(i, 30), &A::Focus).redraw);
            assert_eq!(g.workbench.states[&i].focus, before);
            assert!(!g.accessibility_action(key(i, 30), &A::Press).redraw);
            if renderer == "tree" {
                assert!(
                    !g.accessibility_action(key(i, 30), &A::SetExpanded(false))
                        .redraw
                );
            }
            g.handle_input(&Event::Focus(true));
            g.build_scene(&mut Scene::new());
            assert!(g.accessibility_action(key(i, 30), &A::Press).redraw);
            g.workbench.state(i).disabled = true;
            g.workbench.state(i).extended.disabled = true;
            g.build_scene(&mut Scene::new());
            assert!(!g.accessibility_action(key(i, 30), &A::Press).redraw);
            g.page = Page::Text;
            g.build_scene(&mut Scene::new());
            assert!(!g.accessibility_action(key(i, 30), &A::Press).redraw);
        }
    }
}

#[cfg(test)]
mod search_recovery_focus_tests {
    use super::*;
    use crate::gallery::{Gallery, static_text::StaticText};

    #[test]
    fn native_search_focus_updates_the_workbench_focus_owner() {
        let i = entries().iter().position(|entry| entry.renderer() == "search").unwrap();
        let mut g = Gallery::with_static_text(StaticText::new(None));
        g.page = Page::Workbench(i);
        g.set_view_size(1200., 850.);
        g.build_scene(&mut Scene::new());
        assert!(g.workbench.focus_control(i, 10).is_some());
        assert_eq!(g.workbench.state(i).focus, Some(10));
        let key = g.accessibility_nodes().into_iter()
            .find(|node| node.role == R::SearchField)
            .expect("projected original Search text owner").key;
        assert!(g.accessibility_action(key, &A::Focus).redraw);
        assert_eq!(g.workbench.state(i).focus, Some(1000));
        assert!(g.workbench.state(i).input.search_host().unwrap().focused());
    }
}
