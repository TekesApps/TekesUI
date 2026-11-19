use super::*;
use tekes_ui_controls::composition::ContentEvent;

#[test]
fn apphost_all_registered_routes_mount_real_page_content() {
    let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
    assert_eq!(workbench::entries().len(), 70);
    for entry in workbench::entries() {
        let page = page_host(entry.slug(), &palette).unwrap();
        let notice = page
            .get(ComponentId(202))
            .and_then(|node| node.component::<TextContent>());
        assert!(
            !notice.is_some_and(|copy| copy.text.starts_with("This route has not migrated")),
            "{}",
            entry.slug()
        );
    }
}
#[test]
fn apphost_launcher_navigation_and_repaint_do_not_enqueue_windows() {
    use tekes_ui_controls::composition::AccessibilityAction;
    let state = state("apphost");
    let mut text = super::super::text::TextEngine::new();
    let mut focus = focus_container(state.clone());
    for _ in 0..2 {
        focus.with_host(&mut |host| render_text_host(host, &mut text));
    }
    assert_eq!(state.borrow().window_requests, 0);
    focus.with_host(&mut |host| {
        assert!(
            host.accessibility_action(ComponentId(124020), None, AccessibilityAction::Press)
                .consumed
        );
    });
    assert_eq!(state.borrow().window_requests, 1);
    for _ in 0..2 {
        focus.with_host(&mut |host| render_text_host(host, &mut text));
    }
    assert_eq!(state.borrow().window_requests, 1);
    assert_eq!(state.borrow().visits.len(), 1);
}
#[test]
fn apphost_workbench_consumes_only_its_entry_argument() {
    let args = |values: &[&str]| {
        values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    };
    let launch = args(&["/retained/program/tekes-ui-gallery", "--apphost"]);
    assert_eq!(apphost_arguments(&launch), Some(vec![launch[0].clone()]));
    for tail in [
        vec!["--window-acceptance"],
        vec!["--modules", "unknown"],
        vec!["--worker", "unknown", "socket"],
        vec!["unexpected"],
    ] {
        let mut launch = args(&["gallery", "--apphost"]);
        launch.extend(tail.iter().map(|value| (*value).to_owned()));
        let mut expected = args(&["gallery"]);
        expected.extend(tail.into_iter().map(str::to_owned));
        assert_eq!(apphost_arguments(&launch), Some(expected));
    }
    let default_launch = args(&["gallery"]);
    assert_eq!(
        apphost_arguments(&default_launch),
        Some(default_launch.clone())
    );
    for other in [
        vec!["gallery", "--legacy"],
        vec!["gallery", "--legacy", "--snapshot", "output"],
        vec!["gallery", "--snapshot", "output"],
        vec!["gallery", "--worker", "module", "socket"],
    ] {
        assert!(apphost_arguments(&args(&other)).is_none());
    }
}
fn state(page: &str) -> SharedWorkbench {
    let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
    Rc::new(RefCell::new(WorkbenchState {
        settings: None,
        window_requests: 0,
        palette: palette.clone(),
        current: 0,
        visits: vec![Selection::new(page, None)],
        visit_sections: vec![None],
        pending_section: None,
        pages: HashMap::from([(
            page.into(),
            Rc::new(RefCell::new(page_host(page, &palette).unwrap())),
        )]),
        pending: VecDeque::new(),
        inspected: None,
        document_return_requested: false,
        session_dirs: Vec::new(),
    }))
}
fn render_text_host(host: &mut ContentHost, text: &mut super::super::text::TextEngine) {
    let frame = tekes_ui_scene::Rect::new(0., 0., 740., 900.);
    host.render(
        &mut tekes_ui_scene::Scene::new(),
        text,
        frame,
        frame,
        tekes_ui_tokens::Theme::LIGHT.into(),
    )
    .unwrap();
}
fn render_panel(panel: &mut PanelContent, text: &mut super::super::text::TextEngine) {
    // Same production source and hook/Host sequence. AppHost's own tests
    // separately exercise LocalContainer::render wiring and query ordering.
    panel.prepare_for_render();
    panel.with_host(&mut |host| render_text_host(host, text));
}
fn focus_container(state: SharedWorkbench) -> LocalContainer {
    LocalContainer::from_source("test-focus", FocusContent(state))
}
fn publish(state: &SharedWorkbench, page: &str) {
    state.borrow_mut().navigate(page);
    let entry = state.borrow_mut().pending.pop_front().unwrap();
    let visit = entry.params["visit"].as_u64().unwrap();
    // LocalContainer invokes its command while borrowing the previous host.
    // The shell must not retain a borrow of the application model here.
    focus_container(state.clone()).with_host(&mut |_| {
        state.borrow_mut().restore(COMMAND + visit).unwrap();
    });
}
#[test]
fn apphost_workbench_history_retains_each_visit_and_live_input() {
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let mut focus = focus_container(state.clone());
    let mut text = super::super::text::TextEngine::new();
    focus.with_host(&mut |host| {
        // Formal focus/reveal admission needs the real laid-out text host.
        render_text_host(host, &mut text);
        assert!(host.focus(Some(ComponentId(100)), false));
        render_text_host(host, &mut text);
        assert_eq!(host.focused(), Some(ComponentId(100)));
        host.event_with_text(
            &ContentEvent::Commit {
                text: " edited".into(),
                replacement: None,
            },
            &mut text,
        )
        .unwrap();
        assert!(
            host.text_surface(ComponentId(100))
                .unwrap()
                .text()
                .contains("edited")
        );
    });
    assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
    publish(&state, SEARCH_ROUTE);
    assert_eq!(
        state.borrow().selection().document_id.as_deref(),
        Some("tui.search-field")
    );
    publish(&state, FORM_ROUTE);
    state.borrow_mut().select(Some("published"));
    assert_eq!(
        state.borrow().selection().document_id.as_deref(),
        Some("tui.checkbox")
    );
    state.borrow_mut().restore(COMMAND).unwrap();
    assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
    focus.with_host(&mut |host| {
        assert!(
            host.text_surface(ComponentId(100))
                .unwrap()
                .text()
                .contains("edited")
        )
    });
    state.borrow_mut().restore(COMMAND + 2).unwrap();
    assert_eq!(
        state.borrow().selection().specimen.as_deref(),
        Some("published")
    );
    // Choosing an inspection target never inserts a page visit.
    assert_eq!(state.borrow().visits.len(), 3);
    assert!(state.borrow().pending.is_empty());
}
#[test]
fn apphost_quality_navigation_retains_state_and_consistency_directory() {
    use tekes_ui_controls::composition::AccessibilityAction;
    let state = state("lab-consistency");
    let mut focus = focus_container(state.clone());
    let mut directory = navigation(state.clone()).unwrap();
    let mut text = super::super::text::TextEngine::new();
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.accessibility_action(ComponentId(100101), None, AccessibilityAction::Press)
                .consumed
        );
    });
    let entry = state.borrow_mut().pending.pop_front().unwrap();
    assert_eq!(entry.route, "components-actions");
    let visit = entry.params["visit"].as_u64().unwrap();
    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND + visit).unwrap());
    let component =
        super::super::catalog::component(&super::super::catalog::group("actions").component_ids[0]);
    let chosen = &component.states[1];
    state.borrow_mut().select(Some(component.id.as_str()));
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        let node = host
            .semantics()
            .into_iter()
            .find(|node| node.id == workbench::apphost_quality::PREVIEW)
            .unwrap();
        let part = node
            .parts
            .iter()
            .find(|part| part.semantics.name == format!("{}: {chosen}", component.name))
            .unwrap()
            .index;
        assert!(
            host.accessibility_action(node.id, Some(part), AccessibilityAction::Press)
                .consumed
        );
    });
    let cached = state.borrow().pages["components-actions"].clone();
    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND).unwrap());
    assert_eq!(state.borrow().selection().page, "lab-consistency");
    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND + visit).unwrap());
    assert!(Rc::ptr_eq(
        &cached,
        &state.borrow().pages["components-actions"]
    ));
    focus.with_host(&mut |host| render_text_host(host, &mut text));
    let snapshot = state.borrow().selected_snapshot().unwrap();
    assert!(
        snapshot
            .properties
            .unwrap()
            .iter()
            .any(|property| property.name == component.id && property.value == *chosen)
    );
    directory.with_host(&mut |host| {
        render_text_host(host, &mut text);
        // The full directory is taller than this viewport. Offscreen rows
        // deliberately have no AX parts; selection belongs to the real list.
        let index = workbench::entries()
            .iter()
            .position(|entry| entry.slug() == "lab-consistency")
            .unwrap();
        let section = navigation_sections("")
            .into_iter()
            .find(|section| {
                section
                    .items
                    .iter()
                    .any(|(page, _)| *page == super::super::Page::Workbench(index))
            })
            .unwrap();
        let list = host
            .get(super::super::navigation::Navigation::list_id(&section))
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap();
        assert_eq!(list.selected_key(), Some(ListItemKey(index as u64 + 1)));
    });
    assert_eq!(
        navigation_sections("")
            .iter()
            .map(|section| section.items.len())
            .sum::<usize>(),
        70
    );
    assert_eq!(state.borrow().visits.len(), 2);
}
#[test]
fn apphost_workbench_selection_rejects_retired_specimens_and_visits() {
    let state = state(FORM_ROUTE);
    assert_eq!(state.borrow().selection().specimen, None);
    state.borrow_mut().select(Some("retired-field"));
    assert_eq!(state.borrow().selection().document_id, None);
    let before = state.borrow().selection().clone();
    assert!(state.borrow_mut().restore(COMMAND + 99).is_err());
    assert_eq!(*state.borrow().selection(), before);
    assert!(
        page_host(
            "retired-route",
            &Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)))
        )
        .is_err()
    );
    for route in [SEARCH_ROUTE, FORM_ROUTE] {
        for specimen in specimens(route) {
            assert!(
                workbench::entries()
                    .iter()
                    .any(|entry| Some(entry.component()) == specimen.document_id)
            );
        }
    }
}
#[test]
fn apphost_workbench_filter_retains_registry_identity() {
    let all = navigation_sections("");
    assert_eq!(
        all.iter().map(|section| section.items.len()).sum::<usize>(),
        workbench::entries().len()
    );
    let filtered = navigation_sections("SEARCH-FIELD");
    assert!(!filtered.is_empty());
    for section in filtered {
        let original = all
            .iter()
            .find(|original| original.title == section.title)
            .unwrap();
        assert_eq!(
            super::super::navigation::Navigation::list_id(&section),
            super::super::navigation::Navigation::list_id(original)
        );
        for (page, _) in section.items {
            assert!(
                original
                    .items
                    .iter()
                    .any(|(candidate, _)| *candidate == page)
            );
        }
    }
    assert!(navigation_sections("no-such-gallery-component-291").is_empty());
}
#[test]
fn apphost_workbench_navigation_reuses_group_headers_and_keeps_search_mount() {
    use super::super::navigation::{LIST, Navigation};
    let state = state(SEARCH_ROUTE);
    let mut nav = navigation(state.clone()).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let frame = tekes_ui_scene::Rect::new(0., 60., 300., 700.);
    let mut search_mount = 0;
    nav.with_host(&mut |host| {
        host.render(
            &mut tekes_ui_scene::Scene::new(),
            &mut text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let search = host.get(FILTER).unwrap().frame();
        assert_eq!(search.min_x() - frame.min_x(), Spacing::Loose.value());
        assert_eq!(frame.max_x() - search.max_x(), Spacing::Loose.value());
        assert_eq!(search.min_y() - frame.min_y(), Spacing::Loose.value());
        let rows = host.get(LIST).unwrap().frame();
        assert_eq!(rows.min_x(), search.min_x());
        assert_eq!(rows.max_x(), search.max_x());
        assert!(rows.min_y() > search.max_y());
        for section in super::super::navigation::sections() {
            let list = host
                .get(Navigation::list_id(section))
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap();
            assert_eq!(list.model().snapshot().items.len(), section.items.len());
            for (item, (page, _)) in list.model().snapshot().items.iter().zip(&section.items) {
                assert_eq!(item.name, page.name());
            }
        }
        search_mount = host.get(FILTER).unwrap().mount_identity();
        assert!(host.focus(Some(FILTER), true));
        host.event_with_text(
            &ContentEvent::Commit {
                text: "SearchField".into(),
                replacement: None,
            },
            &mut text,
        )
        .unwrap();
    });
    nav.with_host(&mut |host| {
        assert_eq!(host.get(FILTER).unwrap().mount_identity(), search_mount);
        assert_eq!(host.focused(), Some(FILTER));
        let displayed = navigation_sections("SearchField");
        assert_eq!(
            displayed
                .iter()
                .map(|section| section.items.len())
                .sum::<usize>(),
            1
        );
        let section = &displayed[0];
        assert!(host.get(Navigation::group_id(section)).is_some());
        assert!(
            host.get(Navigation::group_id(
                &super::super::navigation::sections()[0]
            ))
            .is_none()
        );
    });
    assert!(
        state.borrow().pending.is_empty(),
        "filtering must not push navigation history"
    );
}
#[test]
fn apphost_workbench_manual_and_snapshot_follow_exact_instance() {
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    state.borrow_mut().sync_inspection();
    let name = state.borrow().selected_snapshot().unwrap();
    let field_manual = document_body(state.borrow().selection());
    assert!(field_manual.contains("TextField"));
    assert_eq!(
        name.properties
            .as_ref()
            .unwrap()
            .iter()
            .find(|property| property.name == "text")
            .unwrap()
            .value,
        "Gallery"
    );
    state.borrow_mut().select(Some("owner"));
    state.borrow_mut().sync_inspection();
    let owner = state.borrow().selected_snapshot().unwrap();
    assert_eq!(document_body(state.borrow().selection()), field_manual);
    assert_ne!(name.id, owner.id);
    assert_ne!(name.mount, owner.mount);
    assert_eq!(
        owner
            .properties
            .as_ref()
            .unwrap()
            .iter()
            .find(|property| property.name == "text")
            .unwrap()
            .value,
        "TekesUI team"
    );
    assert_eq!(
        state.borrow().inspected,
        Some((FORM_ROUTE.into(), ComponentId(102), owner.mount))
    );
    state.borrow_mut().select(Some("published"));
    state.borrow_mut().sync_inspection();
    assert_eq!(
        state.borrow().selection().document_id.as_deref(),
        Some("tui.checkbox")
    );
    let checkbox_manual = document_body(state.borrow().selection());
    assert!(checkbox_manual.contains("Checkbox"));
    assert_ne!(checkbox_manual, field_manual);
    assert_eq!(
        state.borrow().selected_snapshot().unwrap().id,
        ComponentId(101)
    );
    state.borrow_mut().select(None);
    state.borrow_mut().sync_inspection();
    assert!(state.borrow().selected_snapshot().is_none());
    assert!(state.borrow().inspected.is_none());
    assert!(document_source("retired-component").is_none());
}
#[test]
fn apphost_workbench_inspector_replaces_retired_rows_and_keeps_unchanged_mount() {
    fn rows(host: &ContentHost) -> Vec<(String, String)> {
        host.get(DOCUMENT_READER)
            .unwrap()
            .component::<StackContent>()
            .unwrap()
            .children
            .iter()
            .filter_map(|section| section.component::<KeyValueTableContent>())
            .flat_map(|table| {
                table.cells().chunks_exact(2).map(|pair| {
                    (
                        pair[0].component::<TextContent>().unwrap().text.clone(),
                        pair[1]
                            .component::<TextFieldContent>()
                            .unwrap()
                            .surface
                            .text()
                            .to_owned(),
                    )
                })
            })
            .collect()
    }
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let mut inspector = panel_source(state.clone(), true).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let mut reader_mount = None;
    let mut scroll_mount = None;
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        let rows = rows(host);
        assert!(rows.contains(&("text".into(), "Gallery".into())));
        assert!(rows.iter().any(
            |(key, value)| key == "Current measured layout" && value.starts_with("Unavailable")
        ));
        assert!(
            rows.iter()
                .any(|(key, value)| key == "Recording" && value.starts_with("Not recorded yet"))
        );
        reader_mount = host.semantic_mount(DOCUMENT_READER);
        scroll_mount = host.semantic_mount(DOCUMENT_SCROLL);
    });
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(DOCUMENT_READER), reader_mount);
    });
    state.borrow_mut().select(Some("owner"));
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert_ne!(host.semantic_mount(DOCUMENT_READER), reader_mount);
        assert_eq!(host.semantic_mount(DOCUMENT_SCROLL), scroll_mount);
        let rows = rows(host);
        assert!(rows.contains(&("text".into(), "TekesUI team".into())));
        assert!(!rows.iter().any(|(_, value)| value == "Gallery"));
    });
    state.borrow_mut().select(Some("published"));
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert!(!rows(host).iter().any(|(key, _)| key == "text"));
    });
    // Area is a multiline TextFieldContent and has its property hook.
    // The actual ComboBoxContent mount has no hook, so it exercises the
    // unavailable-properties boundary without inventing a snapshot.
    publish(&state, COMBO_ROUTE);
    {
        let state = state.borrow();
        assert_eq!(
            state.selection().specimen.as_deref(),
            Some("original-combo")
        );
        let snapshot = state
            .selected_snapshot()
            .expect("mounted ComboBox specimen");
        assert_eq!(snapshot.id, ComponentId(27601));
        assert_eq!(
            Some(snapshot.mount),
            state.pages[COMBO_ROUTE]
                .borrow()
                .semantic_mount(snapshot.id),
        );
        assert!(snapshot.properties.is_none());
    }
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        let rows = rows(host);
        assert!(rows.contains(&("Coverage".into(), "Not provided by this component.".into())));
        assert!(!rows.iter().any(|(key, _)| key == "text"));
    });
    state.borrow_mut().select(None);
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        let rows = rows(host);
        assert!(
            rows.iter()
                .any(|(key, value)| key == "Live snapshot" && value.starts_with("Unavailable"))
        );
        assert!(
            !rows
                .iter()
                .any(|(key, _)| key == "Coverage" || key == "Theme" || key == "Frame")
        );
        assert_eq!(host.semantic_mount(DOCUMENT_SCROLL), scroll_mount);
    });
}

#[test]
fn apphost_workbench_inspector_scroll_preserves_mounts_and_skips_unchanged_snapshot() {
    let state = state("component-text");
    state.borrow_mut().select(Some("original"));
    let mut inspector = panel_source(state.clone(), true).unwrap();
    let mut text = super::super::text::TextEngine::new();
    // Populate specimen geometry/paint snapshot once so the Inspector shows full
    // Properties / Observed tokens / Emitted primitives sections.
    focus_container(state.clone()).with_host(&mut |host| {
        render_text_host(host, &mut text);
    });
    render_panel(&mut inspector, &mut text);
    focus_container(state.clone()).with_host(&mut |host| {
        render_text_host(host, &mut text);
    });
    render_panel(&mut inspector, &mut text);

    let (reader_mount, initial_mounts) = {
        let mut out = None;
        inspector.with_host(&mut |host| {
            let mounts: Vec<_> = host
                .semantics()
                .into_iter()
                .map(|node| (node.id, host.semantic_mount(node.id)))
                .collect();
            out = Some((host.semantic_mount(DOCUMENT_READER), mounts));
        });
        out.unwrap()
    };
    assert!(reader_mount.is_some());

    let mut scroll_y = 0.0_f32;
    for step in 1..=6 {
        inspector.with_host(&mut |host| {
            let pos = host.get(DOCUMENT_SCROLL).unwrap().frame().center();
            host.event(&ContentEvent::Scroll {
                position: pos,
                delta: tekes_ui_scene::Point::new(0., 120.),
                precise: true,
                begin: step == 1,
                end: step == 6,
                momentum: false,
                time: step as f64 * 0.016,
            });
        });
        render_panel(&mut inspector, &mut text);
        inspector.with_host(&mut |host| {
            scroll_y = host
                .get(DOCUMENT_SCROLL)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset
                .y;
            assert_eq!(host.semantic_mount(DOCUMENT_READER), reader_mount);
            let mounts: Vec<_> = host
                .semantics()
                .into_iter()
                .map(|node| (node.id, host.semantic_mount(node.id)))
                .collect();
            assert_eq!(mounts, initial_mounts);
        });
    }
    assert!(
        scroll_y > 100.0,
        "Inspector scroll offset must advance across scroll frames (got {scroll_y})"
    );
}
struct InspectorClipboard(RefCell<String>);
impl tekes_ui_module_runtime::ContentClipboard for InspectorClipboard {
    fn clipboard_text(&self) -> Option<String> {
        Some(self.0.borrow().clone())
    }
    fn set_clipboard_text(&self, value: &str) {
        *self.0.borrow_mut() = value.into();
    }
}
fn inspector_value_id(host: &ContentHost, label: &str) -> ComponentId {
    host.get(DOCUMENT_READER)
        .unwrap()
        .component::<StackContent>()
        .unwrap()
        .children
        .iter()
        .filter_map(|section| section.component::<KeyValueTableContent>())
        .flat_map(|table| table.cells().chunks_exact(2))
        .find(|pair| pair[0].component::<TextContent>().unwrap().text == label)
        .unwrap()[1]
        .id()
}
#[test]
fn apphost_workbench_inspector_values_select_copy_and_reject_edits() {
    use tekes_ui_controls::composition::AccessibilityAction;
    use tekes_ui_module_runtime::{ContentClipboard, dispatch_content_window_event_with_text};
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let mut inspector = panel_source(state.clone(), true).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let clipboard = InspectorClipboard(RefCell::new(String::new()));
    focus_container(state.clone()).with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.focus(Some(ComponentId(100)), false));
        render_text_host(host, &mut text);
    });
    render_panel(&mut inspector, &mut text);
    // Actual AppHost mouse-down routing cancels every other local panel
    // before delivering the click. That invalidates specimen layout.
    focus_container(state.clone()).with_host(&mut |host| {
        host.cancel();
        host.focus(None, false);
    });
    assert!(
        state
            .borrow()
            .selected_snapshot()
            .unwrap()
            .geometry
            .is_none()
    );
    let mut value_mount = None;
    inspector.with_host(&mut |host| {
        let value = inspector_value_id(host, "Specimen");
        let point = host.get(value).unwrap().frame().center();
        host.event_with_text(
            &tekes_ui_controls::composition::ContentEvent::PointerDown(point),
            &mut text,
        )
        .unwrap();
        host.event_with_text(
            &tekes_ui_controls::composition::ContentEvent::PointerUp(point),
            &mut text,
        )
        .unwrap();
        assert_eq!(host.focused(), Some(value));
        value_mount = host.semantic_mount(value);
        host.accessibility_action(value, None, AccessibilityAction::SetSelection(0..4));
        assert_eq!(
            host.text_surface(value).unwrap().input().selected_text(),
            "name"
        );
        host.accessibility_action(
            value,
            None,
            AccessibilityAction::ReplaceSelectedText("changed".into()),
        );
        host.accessibility_action(value, None, AccessibilityAction::SetValue("changed".into()));
        assert_eq!(host.text_surface(value).unwrap().text(), "name");
        assert!(host.text_surface(value).unwrap().input().is_read_only());
    });
    // The next ordinary Focus paint restores geometry. Publishing that
    // Layout group must not retire the clicked Identity text owner.
    focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
    assert!(
        state
            .borrow()
            .selected_snapshot()
            .unwrap()
            .geometry
            .is_some()
    );
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        let value = inspector_value_id(host, "Specimen");
        assert_eq!(host.semantic_mount(value), value_mount);
        assert_eq!(host.focused(), Some(value));
        dispatch_content_window_event_with_text(
            host,
            &tekes_ui_platform_macos::Event::TextCommand(
                tekes_ui_platform_macos::TextEditCommand::Copy,
            ),
            &clipboard,
            &mut text,
        )
        .unwrap();
        assert_eq!(clipboard.clipboard_text().as_deref(), Some("name"));
        assert_eq!(
            host.text_surface(inspector_value_id(host, "Specimen"))
                .unwrap()
                .input()
                .selected_text(),
            "name"
        );
    });
    state.borrow_mut().select(Some("owner"));
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        let input = host
            .text_surface(inspector_value_id(host, "Specimen"))
            .unwrap()
            .input();
        assert_eq!(input.text(), "owner");
        assert!(input.selected_text().is_empty());
    });
}
#[test]
fn apphost_workbench_inspector_layout_value_survives_invalidation_and_copies_current_frame() {
    use tekes_ui_controls::composition::{AccessibilityAction, ContentEvent};
    use tekes_ui_module_runtime::{ContentClipboard, dispatch_content_window_event_with_text};
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let mut inspector = panel_source(state.clone(), true).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let clipboard = InspectorClipboard(RefCell::new(String::new()));
    focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
    let current_frame = format!(
        "{:?}",
        state
            .borrow()
            .selected_snapshot()
            .unwrap()
            .geometry
            .unwrap()
            .frame
    );
    render_panel(&mut inspector, &mut text);
    let mut frame_id = ComponentId(0);
    inspector.with_host(&mut |host| {
        frame_id = inspector_value_id(host, "Frame");
        let y = host.get(frame_id).unwrap().frame().min_y();
        host.get_mut(DOCUMENT_SCROLL)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .scroll_by(0., y - 300.);
    });
    render_panel(&mut inspector, &mut text);
    let mut frame_mount = None;
    let mut frame_point = tekes_ui_scene::Point::new(0., 0.);
    inspector.with_host(&mut |host| {
        let frame = host.get(frame_id).unwrap().frame();
        frame_point = tekes_ui_scene::Point::new(frame.min_x() + 8., frame.min_y() + 8.);
        assert!(
            host.get(frame_id)
                .unwrap()
                .visible()
                .unwrap()
                .contains(frame_point)
        );
        frame_mount = host.semantic_mount(frame_id);
    });
    // Real mouse-down ordering: another panel is cancelled before the
    // Inspector receives input. There is no prepare/render between them.
    focus_container(state.clone()).with_host(&mut |host| {
        host.cancel();
        host.focus(None, false);
    });
    assert!(
        state
            .borrow()
            .selected_snapshot()
            .unwrap()
            .geometry
            .is_none()
    );
    inspector.with_host(&mut |host| {
        assert_eq!(host.text_surface(frame_id).unwrap().text(), current_frame);
        host.event_with_text(&ContentEvent::PointerDown(frame_point), &mut text)
            .unwrap();
        host.event_with_text(&ContentEvent::PointerUp(frame_point), &mut text)
            .unwrap();
        assert_eq!(host.focused(), Some(frame_id));
        let len = host.text_surface(frame_id).unwrap().input().len();
        host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..len));
    });
    focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(frame_id), frame_mount);
        assert_eq!(host.focused(), Some(frame_id));
        assert_eq!(
            host.text_surface(frame_id).unwrap().input().selected_text(),
            current_frame
        );
    });
    // If the source really remains invalid at a normal Inspector draw,
    // publish Unavailable honestly and reset only that changed value.
    focus_container(state.clone()).with_host(&mut |host| host.cancel());
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(frame_id), frame_mount);
        assert_eq!(host.focused(), Some(frame_id));
        assert_eq!(host.text_surface(frame_id).unwrap().text(), "Unavailable");
        assert!(
            host.text_surface(frame_id)
                .unwrap()
                .input()
                .selected_text()
                .is_empty()
        );
        host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..11));
    });
    focus_container(state.clone()).with_host(&mut |host| render_text_host(host, &mut text));
    render_panel(&mut inspector, &mut text);
    inspector.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(frame_id), frame_mount);
        assert_eq!(host.focused(), Some(frame_id));
        let input = host.text_surface(frame_id).unwrap().input();
        assert_eq!(input.text(), current_frame);
        assert!(input.selected_text().is_empty());
        let len = input.len();
        host.accessibility_action(frame_id, None, AccessibilityAction::SetSelection(0..len));
        dispatch_content_window_event_with_text(
            host,
            &tekes_ui_platform_macos::Event::TextCommand(
                tekes_ui_platform_macos::TextEditCommand::Copy,
            ),
            &clipboard,
            &mut text,
        )
        .unwrap();
        assert_eq!(
            clipboard.clipboard_text().as_deref(),
            Some(current_frame.as_str())
        );
    });
}
#[test]
fn apphost_workbench_readers_preserve_selection_and_specimen_focus() {
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let unpainted = state.borrow().selected_snapshot().unwrap();
    assert!(unpainted.geometry.is_none());
    assert!(unpainted.last_paint.is_none());
    let body = format!(
        "{:?}",
        inspector_body(state.borrow().selection(), Some(unpainted))
    );
    assert!(body.contains("Unavailable (not rendered or invalidated)"));
    assert!(body.contains("Last paint (historical"));
    assert!(body.contains("Not recorded yet"));
    let mut text = super::super::text::TextEngine::new();
    focus_container(state.clone()).with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.focus(Some(ComponentId(100)), false));
        render_text_host(host, &mut text);
        assert_eq!(host.focused(), Some(ComponentId(100)));
    });
    let mut documents = panel("test-documents", "Documents", state.clone(), false).unwrap();
    let mut inspector = panel_source(state.clone(), true).unwrap();
    let mut version = 0;
    documents.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.text_surface(DOCUMENT_READER)
                .unwrap()
                .input()
                .is_read_only()
        );
        host.accessibility_action(
            DOCUMENT_READER,
            None,
            tekes_ui_controls::composition::AccessibilityAction::SetSelection(0..8),
        );
        version = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap()
            .version()
            .revision();
    });
    render_panel(&mut inspector, &mut text);
    for _ in 0..3 {
        inspector.with_host(&mut |_| {});
        documents.with_host(&mut |host| {
            assert_eq!(
                host.text_surface(DOCUMENT_READER)
                    .unwrap()
                    .input()
                    .selection(),
                0..8
            );
            assert_eq!(
                host.get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .version()
                    .revision(),
                version
            );
        });
    }
    let snapshot = state.borrow().selected_snapshot().unwrap();
    assert!(snapshot.focused);
    assert!(snapshot.geometry.is_some());
    assert!(snapshot.last_paint.is_some());
    let body = format!(
        "{:?}",
        inspector_body(state.borrow().selection(), Some(snapshot))
    );
    assert!(body.contains("Measured bounds, not declared sizing"));
    assert!(body.contains("Last paint (historical"));
}
#[test]
fn apphost_workbench_rich_manual_preserves_code_selection_until_document_switch() {
    use tekes_ui_controls::composition::{AccessibilityAction, RichBlockKind};
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let mut panel = panel("documents", "Documents", state.clone(), false).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let mut mount = None;
    let mut copied = String::new();
    panel.with_host(&mut |host| {
        render_text_host(host, &mut text);
        let rich = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap();
        assert!(
            rich.document()
                .blocks()
                .iter()
                .any(|b| matches!(b.kind(), RichBlockKind::Heading { .. }))
        );
        let code = rich
            .document()
            .blocks()
            .iter()
            .find(|b| b.kind() == RichBlockKind::Code)
            .unwrap()
            .display();
        assert!(!rich.block_geometry().is_empty());
        host.accessibility_action(
            DOCUMENT_READER,
            None,
            AccessibilityAction::SetSelection(code),
        );
        copied = host
            .text_surface(DOCUMENT_READER)
            .unwrap()
            .input()
            .selected_text();
        assert!(copied.contains("TextFieldContent::new"));
        assert!(!copied.contains("```"));
        mount = host.semantic_mount(DOCUMENT_READER);
    });
    state.borrow_mut().select(Some("owner"));
    panel.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
        assert_eq!(
            host.text_surface(DOCUMENT_READER)
                .unwrap()
                .input()
                .selected_text(),
            copied
        );
        let source = format!(
            "{}\n\nUpdated guidance",
            document_source("tui.text-field").unwrap()
        );
        publish_document(host, "tui.text-field", &source, &Cell::new(None));
        assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
        assert_eq!(
            host.text_surface(DOCUMENT_READER)
                .unwrap()
                .input()
                .selected_text(),
            copied
        );
        assert_eq!(
            host.get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .version()
                .revision(),
            2
        );
    });
    state.borrow_mut().select(Some("published"));
    panel.with_host(&mut |host| {
        assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
        let rich = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap();
        assert_eq!(rich.version().identity(), "tui.checkbox");
        assert_eq!(rich.version().revision(), 1);
        assert!(
            host.text_surface(DOCUMENT_READER)
                .unwrap()
                .input()
                .selected_text()
                .is_empty()
        );
    });
}

#[test]
fn apphost_workbench_selected_chooser_activation_returns_linked_reader() {
    use tekes_ui_controls::composition::AccessibilityAction;
    // Both panel choosers publish the same explicit return request.
    for from_inspector in [false, true] {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("owner"));
        let mut documents = panel_source(state.clone(), false).unwrap();
        let mut inspector = panel_source(state.clone(), true).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let mut linked_mount = None;
        documents.with_host(&mut |host| {
            publish_document(
                host,
                "tui.number-field",
                document_source("tui.number-field").unwrap(),
                &Cell::new(None),
            );
            render_text_host(host, &mut text);
            linked_mount = host.semantic_mount(DOCUMENT_READER);
        });
        // A same-value selection or ordinary callback/read is not activation.
        state.borrow_mut().select(Some("owner"));
        consume_chooser_actions(&state, vec![]);
        documents.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(DOCUMENT_READER), linked_mount);
        });
        render_panel(&mut inspector, &mut text);
        let chooser = if from_inspector {
            &mut inspector
        } else {
            &mut documents
        };
        chooser.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let list = host
                .get(CHOOSER)
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap();
            let key = list.selected_key().unwrap();
            let part = list.semantic_part_for_key(key).unwrap();
            assert!(
                host.accessibility_action(CHOOSER, Some(part), AccessibilityAction::Press)
                    .consumed
            );
            let records = host.drain_action_records();
            assert!(records.iter().any(|record| matches!(
                &record.payload,
                Some(ContentActionPayload::ListIntent(intent))
                    if intent.kind == ListIntentKind::Activated
            )));
            consume_chooser_actions(&state, records);
        });
        // Reading Inspector must not consume the Documents request.
        inspector.with_host(&mut |_| {});
        let mut returned_mount = None;
        documents.with_host(&mut |host| {
            returned_mount = host.semantic_mount(DOCUMENT_READER);
            assert_ne!(returned_mount, linked_mount);
            assert_eq!(
                host.get(DOCUMENT_READER)
                    .unwrap()
                    .component::<RichContent>()
                    .unwrap()
                    .version()
                    .identity(),
                "tui.text-field"
            );
        });
        documents.with_host(&mut |host| {
            assert_eq!(host.semantic_mount(DOCUMENT_READER), returned_mount);
        });
        assert_eq!(
            state.borrow().selection().specimen.as_deref(),
            Some("owner")
        );
    }
}

#[test]
fn apphost_workbench_local_rich_link_uses_supplemental_identity_without_page_navigation() {
    use tekes_ui_controls::composition::{AccessibilityAction, Component};
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let selection = state.borrow().selection().clone();
    let mut panel = panel("documents", "Documents", state.clone(), false).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let mut mount = None;
    let document_focus = Cell::new(None);
    let source = RichDocument::from_markdown(document_source("tui.number-field").unwrap());
    let target = source
        .links()
        .iter()
        .find(|link| link.target() == "ticking-number.md")
        .unwrap()
        .target();
    panel.with_host(&mut |host| {
        // Use the real bundled target in a short document so the link is visible.
        publish_document(
            host,
            "tui.number-field",
            &format!("[Ticking Number]({target})"),
            &document_focus,
        );
        render_text_host(host, &mut text);
        mount = host.semantic_mount(DOCUMENT_READER);
        let part = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap()
            .semantic_parts()[0]
            .index;
        assert!(
            host.accessibility_action(DOCUMENT_READER, Some(part), AccessibilityAction::Press)
                .consumed
        );
        assert!(host.drain_action_records().is_empty());
    });
    // Reads/paint must not consume the independent link queue or navigate.
    panel.with_host(&mut |host| {
        assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
        assert_eq!(
            host.get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .version()
                .identity(),
            "tui.number-field"
        );
        consume_document_links(host, &document_focus);
        assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
        let rich = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap();
        assert_eq!(rich.version().identity(), "supplemental.ticking-number");
        assert!(rich.document().source().contains("TickingNumber::new"));
        mount = host.semantic_mount(DOCUMENT_READER);
        consume_document_links(host, &document_focus);
        assert_eq!(
            host.semantic_mount(DOCUMENT_READER),
            mount,
            "link delivered only once"
        );
    });
    state.borrow_mut().select(Some("owner"));
    panel.with_host(&mut |host| {
        assert_ne!(host.semantic_mount(DOCUMENT_READER), mount);
        let rich = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap();
        assert_eq!(rich.version().identity(), "tui.text-field");
        assert_eq!(
            rich.document().source(),
            document_source("tui.text-field").unwrap()
        );
    });
    assert_eq!(state.borrow().selection().page, selection.page);
    assert!(state.borrow().pending.is_empty());
    assert_eq!(
        linked_document("supplemental.ticking-number", "number-field.md")
            .unwrap()
            .0,
        "tui.number-field"
    );
    assert!(document_source("supplemental.ticking-number").is_none());
}

#[test]
fn apphost_workbench_link_reader_focus_waits_for_host_layout_then_accepts_keyboard() {
    use tekes_ui_controls::composition::{
        Component, InputModifiers, PointerButton, PointerEvent, PointerPhase,
    };
    let state = state(FORM_ROUTE);
    state.borrow_mut().select(Some("name"));
    let selection = state.borrow().selection().clone();
    let document_focus = Rc::new(Cell::new(None));
    let mut host = ContentHost::new(stack(
        20,
        vec![
            Content::new(DOCUMENT_NOTICE, TextContent::new(DOCUMENT_HELP)),
            Content::new(ComponentId(24), TextFieldContent::new("Other", "")),
            document_scroll("tui.text-field", "[NumberField](number-field.md)"),
        ],
    ))
    .unwrap();
    host.get_mut(ComponentId(20))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .flexible_child = Some(DOCUMENT_SCROLL);
    let mut panel = PanelContent {
        host,
        state,
        selection: Some(selection),
        inspector: false,
        inspector_groups: Vec::new(),
        inspector_target: None,
        inspector_snapshot: None,
        document_focus: document_focus.clone(),
    };
    let mut text = super::super::text::TextEngine::new();
    panel.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.focus(Some(DOCUMENT_READER), false));
        let point = host
            .get(DOCUMENT_READER)
            .unwrap()
            .component::<RichContent>()
            .unwrap()
            .semantic_parts()[0]
            .frame
            .center();
        assert!(
            host.event(&ContentEvent::Pointer(PointerEvent {
                position: point,
                phase: PointerPhase::Down,
                button: Some(PointerButton::Primary),
                modifiers: InputModifiers {
                    command: true,
                    ..Default::default()
                },
                click_count: 1,
            }))
            .consumed
        );
        consume_document_links(host, &document_focus);
        assert!(document_focus.get().is_some());
        assert!(
            !host.focus(Some(DOCUMENT_READER), false),
            "unarranged replacement must not claim focus"
        );
        assert_eq!(host.focused(), None);
    });
    assert!(
        document_focus.get().is_some(),
        "plain host access cannot complete layout-dependent focus"
    );
    panel.with_host(&mut |host| render_text_host(host, &mut text));
    assert_eq!(document_focus.get(), None);
    panel.with_host(&mut |host| {
        assert_eq!(host.focused(), Some(DOCUMENT_READER));
        assert_eq!(
            host.get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .version()
                .identity(),
            "tui.number-field"
        );
        host.event(&ContentEvent::TextCommand(
            tekes_ui_controls::TextEditCommand::MoveToBeginningOfDocument,
        ));
        host.event(&ContentEvent::TextCommand(
            tekes_ui_controls::TextEditCommand::MoveToEndOfDocumentAndModifySelection,
        ));
        let input = host.text_surface(DOCUMENT_READER).unwrap().input();
        assert_eq!(input.selected_text(), input.text());
        assert!(!input.selected_text().is_empty());
        // A user choosing another focus owner before the transfer completes
        // must not have that choice overwritten by the pending reader.
        publish_document(
            host,
            "tui.text-field",
            document_source("tui.text-field").unwrap(),
            &document_focus,
        );
        render_text_host(host, &mut text);
        assert!(host.focus(Some(ComponentId(24)), false));
    });
    assert_eq!(document_focus.get(), None);
    panel.with_host(&mut |host| assert_eq!(host.focused(), Some(ComponentId(24))));
}

#[test]
fn apphost_workbench_unbundled_link_reports_without_replacing_reader() {
    use tekes_ui_controls::composition::{AccessibilityAction, Component};
    let mut panel = panel("documents", "Documents", state(FORM_ROUTE), false).unwrap();
    let mut text = super::super::text::TextEngine::new();
    let document_focus = Cell::new(None);
    for target in ["https://example.com/manual", "missing.md"] {
        panel.with_host(&mut |host| {
            publish_document(
                host,
                "tui.number-field",
                &format!("[Manual]({target})"),
                &document_focus,
            );
            render_text_host(host, &mut text);
            let mount = host.semantic_mount(DOCUMENT_READER);
            let part = host
                .get(DOCUMENT_READER)
                .unwrap()
                .component::<RichContent>()
                .unwrap()
                .semantic_parts()[0]
                .index;
            host.accessibility_action(DOCUMENT_READER, Some(part), AccessibilityAction::Press);
            consume_document_links(host, &document_focus);
            assert_eq!(host.semantic_mount(DOCUMENT_READER), mount);
            let notice = &host
                .get(DOCUMENT_NOTICE)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text;
            assert!(notice.contains("Link not opened"));
            assert!(notice.contains(target));
        });
    }
}
#[test]
fn apphost_workbench_reenables_inspection_after_same_id_remount() {
    use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
    use tekes_ui_scene::{Rect, Scene, Size};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, role: TextRole, width: f32) -> Size {
            Size::new(60_f32.min(width), role.style().line_height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    let state = state(FORM_ROUTE);
    // Keep only the selected original control so this regression needs no
    // native text resources. Replacements use the public host transaction.
    state.borrow_mut().pages.insert(
        FORM_ROUTE.into(),
        Rc::new(RefCell::new(
            ContentHost::new(stack(
                200,
                vec![Content::new(
                    ComponentId(101),
                    CheckboxContent::new("Published", CheckState::Unchecked),
                )],
            ))
            .unwrap()
            .into(),
        )),
    );
    state.borrow_mut().select(Some("published"));
    let render = |state: &SharedWorkbench| {
        state.borrow_mut().with_selected_host(&mut |host, _| {
            let frame = Rect::new(0., 0., 240., 100.);
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
        });
    };
    state.borrow_mut().sync_inspection();
    render(&state);
    let original = state.borrow().selected_snapshot().unwrap();
    assert!(original.last_paint.is_some());
    let selection = state.borrow().selection().clone();
    state
        .borrow_mut()
        .with_selected_host(&mut |host, specimen| {
            host.remove(specimen.component_id).unwrap();
            host.insert(
                ComponentId(200),
                Content::new(
                    specimen.component_id,
                    CheckboxContent::new("Replacement", CheckState::Checked),
                ),
            )
            .unwrap();
        });
    render(&state);
    let replacement = state.borrow().selected_snapshot().unwrap();
    assert_ne!(replacement.mount, original.mount);
    assert!(replacement.last_paint.is_none());
    state.borrow_mut().sync_inspection();
    render(&state);
    assert!(
        state
            .borrow()
            .selected_snapshot()
            .unwrap()
            .last_paint
            .is_some()
    );
    assert_eq!(*state.borrow().selection(), selection);
    state
        .borrow_mut()
        .with_selected_host(&mut |host, specimen| {
            host.remove(specimen.component_id).unwrap();
        });
    state.borrow_mut().sync_inspection();
    assert!(state.borrow().inspected.is_none());
    assert!(state.borrow().selected_snapshot().is_none());
}
#[test]
fn apphost_display_progress_and_nested_panel_actions_keep_original_owners() {
    use tekes_ui_controls::composition::{AccessibilityAction, Button};
    for route in ["component-progress", "component-surface"] {
        let state = state(route);
        let mut focus = focus_container(state);
        let mut text = super::super::text::TextEngine::new();
        let count = if route == "component-progress" { 4 } else { 1 };
        for _ in 0..count {
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                assert!(
                    host.accessibility_action(ComponentId(92120), None, AccessibilityAction::Press)
                        .consumed
                );
            });
        }
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                !host
                    .accessibility_action(ComponentId(96020), None, AccessibilityAction::Press)
                    .consumed,
                "passive owner is not the nested action"
            );
            let log = &host
                .get(ComponentId(92003))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text;
            if route == "component-progress" {
                assert_eq!(log.matches("progress:").count(), 4);
                assert_eq!(
                    host.get(ComponentId(96021))
                        .unwrap()
                        .component::<TextContent>()
                        .unwrap()
                        .text,
                    "100%"
                );
                assert!(
                    host.get(ComponentId(92120))
                        .unwrap()
                        .component::<Button>()
                        .unwrap()
                        .button
                        .state
                        .disabled
                );
                assert!(
                    !host
                        .accessibility_action(ComponentId(92120), None, AccessibilityAction::Press)
                        .consumed
                );
            } else {
                assert_eq!(log.matches("activate: primary action").count(), 1);
            }
        });
    }
}

#[test]
fn apphost_section_bar_keeps_one_row_and_reveals_last_command_at_narrow_width() {
    use tekes_ui_controls::composition::AccessibilityAction;
    for width in [740., 280.] {
        let state = state("component-button");
        let mut focus = focus_container(state);
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let frame = tekes_ui_scene::Rect::new(0., 0., width, 900.);
        focus.with_host(&mut |host| {
            host.render(
                &mut tekes_ui_scene::Scene::new(),
                &mut text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            let first = host.get(ComponentId(92101)).unwrap().frame();
            let last = host.get(ComponentId(92100)).unwrap().frame();
            assert!(
                (first.min_y() - last.min_y()).abs() < 0.1,
                "section commands formed a vertical column"
            );
            assert!(
                host.get(ComponentId(92006)).unwrap().frame().size.height < first.size.height * 2.
            );
            assert_eq!(
                host.get(ComponentId(92004))
                    .unwrap()
                    .component::<TextContent>()
                    .unwrap()
                    .role,
                TextRole::UiHeading
            );
            assert!(host.focus(Some(ComponentId(92100)), true));
            host.render(
                &mut tekes_ui_scene::Scene::new(),
                &mut text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            let last = host.get(ComponentId(92100)).unwrap().frame();
            let viewport = host.get(ComponentId(92012)).unwrap().frame();
            assert!(
                last.min_x() >= viewport.min_x() - 0.1 && last.max_x() <= viewport.max_x() + 0.1,
                "Reset remains outside the horizontal viewport"
            );
            assert!(
                host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                    .consumed
            );
        });
    }
}

#[test]
fn apphost_reading_pages_measure_original_documents_inside_narrow_focus() {
    use tekes_ui_controls::composition::AccessibilityAction;
    for route in [
        "component-markdown",
        "component-code-block",
        "component-diff-view",
    ] {
        let state = state(route);
        let mut focus = focus_container(state);
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let frame = tekes_ui_scene::Rect::new(0., 0., 320., 900.);
        focus.with_host(&mut |host| {
            host.render(
                &mut tekes_ui_scene::Scene::new(),
                &mut text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            let document = host.get(ComponentId(96020)).unwrap().frame();
            let footer = host.get(ComponentId(96021)).unwrap().frame();
            assert!(document.size.height > 0. && document.size.width <= frame.size.width);
            assert!(
                footer.min_y() >= document.max_y(),
                "document text collides with its original footer"
            );
            assert!(
                !host
                    .accessibility_action(ComponentId(96020), None, AccessibilityAction::Press)
                    .consumed
            );
        });
    }
}

#[test]
fn apphost_selection_pages_preserve_real_changes_across_passive_sections() {
    use tekes_ui_controls::composition::{
        AccessibilityAction, Component, RadioGroupContent, SliderContent, SwitchContent,
    };
    for route in [
        "component-switch",
        "component-slider",
        "component-radio-group",
    ] {
        let state = state(route);
        let mut focus = focus_container(state);
        let mut text = super::super::text::TextEngine::new();
        let specimen = workbench::apphost_basic::SPECIMEN;
        let action = if route == "component-slider" {
            AccessibilityAction::Increment
        } else {
            AccessibilityAction::Press
        };
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let part = host
                .get(specimen)
                .unwrap()
                .component::<RadioGroupContent>()
                .map(|radio| radio.semantic_parts()[1].index);
            assert!(
                host.accessibility_action(specimen, part, action.clone())
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(
                host.accessibility_action(ComponentId(92110), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let part = host
                .get(specimen)
                .unwrap()
                .component::<RadioGroupContent>()
                .and_then(|radio| radio.semantic_parts().get(1).map(|part| part.index));
            assert!(
                !host
                    .accessibility_action(specimen, part, action.clone())
                    .consumed
            );
            assert!(
                host.accessibility_action(ComponentId(92103), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.get(specimen).is_none());
            assert!(
                !host
                    .accessibility_action(ComponentId(92801), None, AccessibilityAction::Press)
                    .consumed
            );
            assert!(
                host.accessibility_action(ComponentId(92102), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let content = host.get(specimen).unwrap();
            match route {
                "component-switch" => {
                    let c = content.component::<SwitchContent>().unwrap();
                    assert!(c.model.checked && c.model.disabled);
                }
                "component-slider" => {
                    let c = content.component::<SliderContent>().unwrap();
                    assert_eq!(c.model.value(), 55.);
                    assert!(c.model.disabled);
                }
                _ => assert_eq!(
                    content
                        .component::<RadioGroupContent>()
                        .unwrap()
                        .model()
                        .selected,
                    Some(1)
                ),
            }
            assert!(
                host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                    .consumed
            );
        });
        focus.with_host(&mut |host| {
            render_text_host(host, &mut text);
            let content = host.get(specimen).unwrap();
            match route {
                "component-switch" => {
                    let c = content.component::<SwitchContent>().unwrap();
                    assert!(!c.model.checked && !c.model.disabled);
                }
                "component-slider" => {
                    let c = content.component::<SliderContent>().unwrap();
                    assert_eq!(c.model.value(), 50.);
                    assert!(!c.model.disabled);
                }
                _ => assert_eq!(
                    content
                        .component::<RadioGroupContent>()
                        .unwrap()
                        .model()
                        .selected,
                    Some(0)
                ),
            }
        });
    }
}

#[test]
fn apphost_navigation_deactivates_departing_cached_host_in_its_existing_borrow() {
    let state = state(FORM_ROUTE);
    let old = state.borrow().pages[FORM_ROUTE].clone();
    assert!(old.borrow().is_window_active());
    publish(&state, SEARCH_ROUTE);
    assert!(!old.borrow().is_window_active());
    assert!(
        state.borrow().pages[SEARCH_ROUTE]
            .borrow()
            .is_window_active()
    );
    state.borrow_mut().restore(COMMAND).unwrap();
    let mut text = super::super::text::TextEngine::new();
    focus_container(state).with_host(&mut |host| render_text_host(host, &mut text));
    assert!(old.borrow().is_window_active());
}

#[test]
fn apphost_scene3d_specimen_uses_its_page_guidance_without_a_fictitious_manual() {
    let selection = Selection::new("component-scene3d", None);
    assert_eq!(selection.specimen.as_deref(), Some("original-scene3d"));
    assert!(selection.document_id.is_none());
    assert!(document_body(&selection).contains("glTF"));
    assert!(!document_body(&selection).contains("has not migrated"));
    assert_eq!(document_key(&selection), "page:component-scene3d");
    assert!(
        page_host(
            "component-scene3d",
            &Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)))
        )
        .unwrap()
        .get(workbench::apphost_scene3d::SPECIMEN)
        .is_some()
    );
}

#[test]
fn apphost_button_loading_rejects_action_and_sections_preserve_fixture_state() {
    use tekes_ui_controls::composition::{AccessibilityAction, Button};
    let state = state("component-button");
    let mut focus = focus_container(state);
    let mut text = super::super::text::TextEngine::new();
    let specimen = workbench::apphost_basic::SPECIMEN;
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.accessibility_action(specimen, None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        let log = &host
            .get(ComponentId(92003))
            .unwrap()
            .component::<TextContent>()
            .unwrap()
            .text;
        assert_eq!(log.matches("activate: primary action").count(), 1);
        render_text_host(host, &mut text);
        assert!(
            host.accessibility_action(ComponentId(92111), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.get(specimen)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .loading
        );
        assert!(
            !host
                .accessibility_action(specimen, None, AccessibilityAction::Press)
                .consumed
        );
        assert!(
            host.accessibility_action(ComponentId(92106), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.get(specimen).is_none());
        // The original sized examples are passive; no accidental action
        // owner is introduced when wrapping the old Button renderer.
        assert!(
            !host
                .accessibility_action(ComponentId(92400), None, AccessibilityAction::Press)
                .consumed
        );
        assert!(
            host.accessibility_action(ComponentId(92105), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.get(specimen)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .loading
        );
        assert!(
            host.accessibility_action(ComponentId(92111), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert_eq!(
            host.get(specimen)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .label,
            "A long action label 中日文"
        );
        assert!(
            host.accessibility_action(specimen, None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        let log = &host
            .get(ComponentId(92003))
            .unwrap()
            .component::<TextContent>()
            .unwrap()
            .text;
        assert_eq!(log.matches("activate: primary action").count(), 2);
    });
}

#[test]
fn apphost_index_activation_enters_the_original_registry_route_once() {
    use tekes_ui_controls::composition::AccessibilityAction;
    let state = state("component-index");
    let mut focus = focus_container(state.clone());
    let mut text = super::super::text::TextEngine::new();
    let index = workbench::entries()
        .iter()
        .position(|e| e.slug() == "component-stack")
        .unwrap();
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(
            host.accessibility_action(
                ComponentId(94000 + index as u64),
                None,
                AccessibilityAction::Press
            )
            .consumed
        );
    });
    let request = FocusContent(state.clone())
        .take_navigation_request()
        .unwrap();
    assert!(
        FocusContent(state.clone())
            .take_navigation_request()
            .is_none()
    );
    let visit = request.params["visit"].as_u64().unwrap();
    state.borrow_mut().restore(COMMAND + visit).unwrap();
    assert_eq!(state.borrow().selection().page, "component-stack");
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.get(ComponentId(92601)).is_some());
        assert!(
            host.accessibility_action(ComponentId(92106), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.get(ComponentId(92600)).is_some());
        assert!(host.get(ComponentId(92601)).is_some());
        assert!(host.get(ComponentId(92602)).is_some());
        assert!(
            !host
                .accessibility_action(ComponentId(92601), None, AccessibilityAction::Press)
                .consumed
        );
    });
}

#[test]
fn apphost_checkbox_sections_preserve_live_model_and_disable_without_remount() {
    use tekes_ui_controls::composition::AccessibilityAction;
    let state = state("component-checkbox");
    let mut focus = focus_container(state.clone());
    let mut text = super::super::text::TextEngine::new();
    let specimen = workbench::apphost_basic::SPECIMEN;
    {
        let mut press = |target| {
            focus.with_host(&mut |host| {
                render_text_host(host, &mut text);
                assert!(
                    host.accessibility_action(target, None, AccessibilityAction::Press)
                        .consumed
                );
            });
        };
        // Mixed is a fixture command; activating the real checkbox must toggle
        // exactly once, not replay through Gallery's old activation reducer.
        press(ComponentId(92113));
        press(specimen);
    }
    focus.with_host(&mut |host| {
        assert_eq!(
            host.get(specimen)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap()
                .model()
                .checked,
            CheckState::Checked
        );
    });
    let mut mount = None;
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        mount = host.semantic_mount(specimen);
        assert!(
            host.accessibility_action(ComponentId(92110), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert_eq!(host.semantic_mount(specimen), mount);
        assert!(
            !host
                .accessibility_action(specimen, None, AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(
            host.get(specimen)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap()
                .model()
                .checked,
            CheckState::Checked
        );
        assert!(
            host.accessibility_action(ComponentId(92103), None, AccessibilityAction::Press)
                .consumed
        );
    });
    // The state matrix is passive and retires the interactive mount.
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.get(specimen).is_none());
        assert!(
            !host
                .accessibility_action(specimen, None, AccessibilityAction::Press)
                .consumed
        );
        assert!(
            host.accessibility_action(ComponentId(92105), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        render_text_host(host, &mut text);
        let control = host
            .get(specimen)
            .unwrap()
            .component::<CheckboxContent>()
            .unwrap();
        assert_eq!(control.model().checked, CheckState::Checked);
        assert!(control.model().button.state.disabled);
        assert_ne!(host.semantic_mount(specimen), mount);
    });
    // Leave the route and return: the live section/model owner is retained.
    publish(&state, SEARCH_ROUTE);
    state.borrow_mut().restore(COMMAND).unwrap();
    focus.with_host(&mut |host| {
        assert_eq!(
            host.get(ComponentId(92007))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text,
            "Edge cases"
        );
        assert_eq!(
            host.get(specimen)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap()
                .model()
                .checked,
            CheckState::Checked
        );
        render_text_host(host, &mut text);
        assert!(
            host.accessibility_action(ComponentId(92100), None, AccessibilityAction::Press)
                .consumed
        );
    });
    focus.with_host(&mut |host| {
        let control = host
            .get(specimen)
            .unwrap()
            .component::<CheckboxContent>()
            .unwrap();
        assert_eq!(control.model().checked, CheckState::Unchecked);
        assert!(!control.model().button.state.disabled);
        assert_eq!(
            host.get(ComponentId(92007))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text,
            "Edge cases"
        );
    });
}

#[test]
fn apphost_original_pages_preserve_edits_and_specimen_selection_on_history() {
    let state = state(AREA_ROUTE);
    let mut text = super::super::text::TextEngine::new();
    focus_container(state.clone()).with_host(&mut |host| {
        render_text_host(host, &mut text);
        assert!(host.focus(Some(ComponentId(27502)), false));
        render_text_host(host, &mut text);
        host.event_with_text(
            &ContentEvent::Commit {
                text: " retained area edit".into(),
                replacement: None,
            },
            &mut text,
        )
        .unwrap();
    });
    let original_mount = state.borrow().selected_snapshot().unwrap().mount;
    for route in [COMBO_ROUTE, SELECT_ROUTE, FORM_ROUTE] {
        publish(&state, route);
    }
    state.borrow_mut().select(Some("original-field"));
    assert_eq!(
        state.borrow().selected_snapshot().unwrap().id,
        ComponentId(27501)
    );
    state.borrow_mut().restore(COMMAND).unwrap();
    assert_eq!(
        state.borrow().selection().document_id.as_deref(),
        Some("tui.text-area")
    );
    focus_container(state.clone()).with_host(&mut |host| {
        assert!(
            host.text_surface(ComponentId(27502))
                .unwrap()
                .text()
                .contains("retained area edit")
        );
    });
    assert_eq!(
        state.borrow().selected_snapshot().unwrap().mount,
        original_mount
    );
    state.borrow_mut().restore(COMMAND + 3).unwrap();
    assert_eq!(
        state.borrow().selection().specimen.as_deref(),
        Some("original-field")
    );
}

#[test]
fn apphost_supports_slug_and_story_startup_routes() {
    use tekes_ui_controls::composition::{IconImage, IconRenderError, WithIconRenderer};
    use tekes_ui_controls::layout_view::IconRenderer;

    struct TestIcons;
    impl IconRenderer for TestIcons {
        fn resource_identity(&self) -> Option<u64> {
            Some(1)
        }
        fn register_icon_bitmap(
            &mut self,
            _: &[u8],
            _: u32,
            _: u32,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(tekes_ui_scene::ImageId(1), Rc::new(())))
        }
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: tekes_ui_tokens::Rgba,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(tekes_ui_scene::ImageId(2), Rc::new(())))
        }
        fn draw_icon(
            &mut self,
            _: &mut tekes_ui_scene::Scene,
            _: &str,
            _: tekes_ui_scene::Rect,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }

    let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
    for (raw, canonical_page) in [
        ("component-button/playground", "component-button"),
        ("component-dialog/behaviors", "component-dialog"),
        ("example-settings", "example-settings"),
        ("example-settings/edge-cases", "example-settings"),
        ("component-text-field/states", "component-text-field"),
        ("component-icon/docs", "component-icon"),
        ("component-text/behaviors", "component-text"),
        ("component-scene3d/docs", "component-scene3d"),
        ("tui.button/variants", "component-button"),
        ("components-actions", "components-actions"),
    ] {
        let (app, session_dir) = assemble_for_route(raw)
            .unwrap_or_else(|e| panic!("expected route {raw} to assemble: {e:#}"));
        drop(app);
        let _ = std::fs::remove_dir_all(&session_dir);

        let (resolved, _) =
            open_route_host(raw, &palette).unwrap_or_else(|e| panic!("{raw}: {e:#}"));
        assert_eq!(resolved, canonical_page);
    }

    let mut text = super::super::text::TextEngine::new();
    text.set_motion_enabled(true);
    text.set_motion_context(std::time::Duration::ZERO, true);
    let mut icons = TestIcons;
    let frame = tekes_ui_scene::Rect::new(0., 0., 740., 900.);
    for entry in workbench::entries() {
        for story in [
            "docs",
            "playground",
            "states",
            "behaviors",
            "edge-cases",
            "variants",
        ] {
            let route = format!("{}/{story}", entry.slug());
            if entry.supports_story(story) {
                let (resolved, mut mounted) =
                    open_route_host(&route, &palette).unwrap_or_else(|e| panic!("{route}: {e:#}"));
                assert_eq!(resolved, entry.slug());
                mounted
                    .render(
                        &mut tekes_ui_scene::Scene::new(),
                        &mut WithIconRenderer::new(&mut text, &mut icons),
                        frame,
                        frame,
                        tekes_ui_tokens::Theme::LIGHT.into(),
                    )
                    .unwrap_or_else(|e| panic!("{route} render: {e:?}"));
            } else {
                assert!(
                    open_route_host(&route, &palette).is_err(),
                    "{route} must fail"
                );
            }
        }
    }

    for invalid in [
        "unknown-route",
        "component-button/",
        "component-button/unknown",
        "component-button/docs/extra",
        "foundation-tokens/states",
        "component-scene3d/behaviors",
        "component-stack/edge-cases",
        "components-actions/states",
    ] {
        let err = open_route_host(invalid, &palette)
            .err()
            .expect("invalid route must fail");
        assert!(
            err.to_string().contains("unknown Gallery route"),
            "unexpected error for {invalid}: {err:#}"
        );
    }
}

#[test]
fn apphost_focus_restore_inside_borrowed_host_handles_initial_and_same_page_sections() {
    let (app, state, session_dir) =
        assemble_for_route_internal("component-button/playground", true).unwrap();
    drop(app);
    let mut focus = focus_container(state.clone());
    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND).unwrap());
    focus.with_host(&mut |h| assert!(h.get(workbench::apphost_basic::SPECIMEN).is_some()));

    state.borrow_mut().navigate("component-button/states");
    let entry = state.borrow_mut().pending.pop_front().unwrap();
    let visit = entry.params["visit"].as_u64().unwrap();
    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND + visit).unwrap());
    focus.with_host(&mut |h| assert!(h.get(workbench::apphost_basic::SPECIMEN).is_none()));

    focus.with_host(&mut |_| state.borrow_mut().restore(COMMAND).unwrap());
    focus.with_host(&mut |h| assert!(h.get(workbench::apphost_basic::SPECIMEN).is_some()));
    let _ = std::fs::remove_dir_all(&session_dir);
}
