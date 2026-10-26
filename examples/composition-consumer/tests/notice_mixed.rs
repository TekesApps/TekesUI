#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::{
    empty_state_content::{self as fixture, MESSAGE},
    scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER},
};
use tekes_ui_controls::{
    ControlKey,
    composition::*,
    extended::{Banner, NoticeTone, Toast},
};
use tekes_ui_platform_macos::{
    AccessibilityAction as NativeAction,
    content_accessibility::{content_action, content_key},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::surface_names};
const NOTICE: ComponentId = ComponentId(79010);
const DISMISS: ComponentId = ComponentId(79011);
const BANNER: ComponentId = ComponentId(79012);
const BANNER_ACTION: ComponentId = ComponentId(79013);
fn render(h: &mut ContentHost, t: &mut coretext::Text, p: Palette) {
    t.services = true;
    t.atlas.begin_frame();
    let f = Rect::new(0.25, 0.5, 360., 1000.);
    h.render(&mut Scene::new(), t, f, f, p).unwrap();
}
#[test]
fn default_mixed() {
    mixed(false);
}
#[test]
#[ignore = "explicit external input lane"]
fn external_mixed() {
    mixed(true);
}
fn mixed(external: bool) {
    let (mut s, mut version) = fixture::build().unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(
                NOTICE,
                ToastContent::new(
                    "Saved",
                    Toast::new("Saved", NoticeTone::Success, 5000),
                    DISMISS,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(
                BANNER,
                BannerContent::new(
                    "Banner",
                    Banner {
                        message: "Banner".into(),
                        tone: NoticeTone::Warning,
                        dismissible: true,
                        visible: true,
                    },
                    BANNER_ACTION,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    s.host
        .reorder(ROOT, &[SCROLLER, MESSAGE, NOTICE, BANNER, LATEST, COMPOSER])
        .unwrap();
    let mut t = coretext::Text::new(1.);
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    let editor = s.host.semantic_mount(COMPOSER);
    let root = s.host.semantic_mount(MESSAGE);
    let keys = s.item_keys();
    let items: Vec<_> = keys.iter().map(|id| s.host.semantic_mount(*id)).collect();
    let input = format!("{:?}", s.host.focused_text().unwrap().input);
    s.host.drain_actions();
    for theme in Theme::ALL {
        for surface in surface_names() {
            let palettes = if external {
                vec![
                    Palette::for_theme_definition(theme, "blue").unwrap(),
                    Palette::for_theme_definition(theme, "blue-inputs").unwrap(),
                ]
            } else {
                vec![Palette::new(theme, 2).unwrap()]
            };
            for palette in palettes {
                for action in [Some("Create"), None, Some("Open 文档")] {
                    let (v, ()) = s
                        .host
                        .update_scroll_reading_document(
                            SCROLLER,
                            &version,
                            version.revision() + 1,
                            keys.clone(),
                            |h| {
                                h.show_toast(
                                    NOTICE,
                                    action.unwrap_or(""),
                                    NoticeTone::Warning,
                                    5000,
                                )
                                .unwrap();
                                h.replace_banner(
                                    BANNER,
                                    Banner {
                                        message: "Changed banner".into(),
                                        tone: NoticeTone::Error,
                                        dismissible: action.is_some(),
                                        visible: true,
                                    },
                                )
                                .unwrap();
                                h.set_inherited_surface(surface).unwrap();
                            },
                        )
                        .unwrap();
                    version = v;
                    render(&mut s.host, &mut t, palette);
                    let after = s.host.text_input_snapshot().unwrap();
                    assert_eq!(after.binding, before.binding);
                    assert_eq!(after.text, before.text);
                    assert_eq!(after.selection, before.selection);
                    assert_eq!(after.marked, before.marked);
                    assert_eq!(s.host.focused(), Some(COMPOSER));
                    assert_eq!(s.host.focused_text_binding(), binding);
                    assert_eq!(s.host.focus_epoch(), epoch);
                    assert_eq!(s.host.semantic_mount(COMPOSER), editor);
                    assert_eq!(s.host.semantic_mount(MESSAGE), root);
                    assert_eq!(
                        keys.iter()
                            .map(|id| s.host.semantic_mount(*id))
                            .collect::<Vec<_>>(),
                        items
                    );
                    assert_eq!(format!("{:?}", s.host.focused_text().unwrap().input), input);
                    s.host.event(&ContentEvent::Key {
                        key: ControlKey::Enter,
                        down: true,
                    });
                    assert!(s.host.drain_actions().is_empty());
                }
            }
        }
    }
    // Retained instances reorder without inventing a remount or editor update.
    s.host
        .reorder(ROOT, &[SCROLLER, LATEST, MESSAGE, NOTICE, BANNER, COMPOSER])
        .unwrap();
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert_eq!(s.host.focused_text_binding(), binding);
    assert_eq!(s.host.semantic_mount(COMPOSER), editor);
}

#[test]
fn actual_native_keys_and_exposure_replacement() {
    let mut h = ContentHost::new(Content::new(
        NOTICE,
        ToastContent::new(
            "Toast",
            Toast::new("Saved", NoticeTone::Success, 5000),
            DISMISS,
        )
        .unwrap(),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let old = content_key(&h, DISMISS, None).unwrap();
    let frame = h.get(DISMISS).unwrap().frame();
    h.event(&ContentEvent::PointerDown(frame.center()));
    h.show_toast(NOTICE, "Saved", NoticeTone::Success, 5000)
        .unwrap();
    assert!(h.pointer_capture().is_none());
    assert!(!h.event(&ContentEvent::PointerUp(frame.center())).activated);
    assert!(!content_action(&mut h, old, NativeAction::Press).activated);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let key = content_key(&h, DISMISS, None).unwrap();
    assert_ne!(old, key);
    assert!(!content_action(&mut h, old, NativeAction::Press).activated);
    assert!(content_action(&mut h, key, NativeAction::Press).activated);
    assert_eq!(h.drain_actions().len(), 1);
    assert!(!content_action(&mut h, key, NativeAction::Press).activated);
    h.show_toast(NOTICE, "New", NoticeTone::Info, 5000).unwrap();
    render(&mut h, &mut t, Theme::LIGHT.into());
    let key = content_key(&h, DISMISS, None).unwrap();
    assert!(content_action(&mut h, key, NativeAction::Press).activated);
    h.show_toast(NOTICE, "New", NoticeTone::Info, 5000).unwrap();
    assert!(h.drain_actions().is_empty());
    render(&mut h, &mut t, Theme::LIGHT.into());
    let key = content_key(&h, DISMISS, None).unwrap();
    h.set_window_active(false);
    h.set_window_active(true);
    assert!(!content_action(&mut h, key, NativeAction::Press).activated);
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert_ne!(content_key(&h, DISMISS, None).unwrap(), key);
}

#[test]
fn actual_mixed_composer_narrow_partial_full_clip_and_recovery() {
    let (mut s, version) = fixture::build().unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(
                NOTICE,
                ToastContent::new(
                    "Toast",
                    Toast::new("Saved", NoticeTone::Success, 5000),
                    DISMISS,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(
                BANNER,
                BannerContent::new(
                    "Banner",
                    Banner {
                        message: "Banner".into(),
                        tone: NoticeTone::Warning,
                        dismissible: true,
                        visible: true,
                    },
                    BANNER_ACTION,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    s.host
        .reorder(ROOT, &[SCROLLER, MESSAGE, NOTICE, BANNER, LATEST, COMPOSER])
        .unwrap();
    let mut t = coretext::Text::new(1.);
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    let mount = s.host.semantic_mount(COMPOSER);
    let keys = s.item_keys();
    let mounts = keys
        .iter()
        .map(|id| s.host.semantic_mount(*id))
        .collect::<Vec<_>>();
    for theme in Theme::ALL {
        for surface in surface_names() {
            let palette = if std::env::var_os("TEKESUI_EMPTY_EXTERNAL").is_some() {
                Palette::for_theme_definition(theme, "blue-inputs").unwrap()
            } else {
                Palette::new(theme, 2).unwrap()
            };
            s.host
                .show_toast(NOTICE, "Saved 更改已保存 👩🏽‍💻", NoticeTone::Warning, 5000)
                .unwrap();
            s.host.set_inherited_surface(surface).unwrap();
            for (frame, clip) in [
                (
                    Rect::new(0.25, 0.5, 240., 1000.),
                    Rect::new(30., 40., 180., 830.),
                ),
                (
                    Rect::new(0.25, 0.5, 240., 1000.),
                    Rect::new(1800., 1800., 10., 10.),
                ),
                (
                    Rect::new(0.25, 0.5, 360., 1000.),
                    Rect::new(0., 0., 400., 1040.),
                ),
            ] {
                t.atlas.begin_frame();
                s.host
                    .render(&mut Scene::new(), &mut t, frame, clip, palette)
                    .unwrap();
                let after = s.host.text_input_snapshot().unwrap();
                assert_eq!(after.text, before.text);
                assert_eq!(after.selection, before.selection);
                assert_eq!(after.marked, before.marked);
                assert_eq!(after.binding, before.binding);
                assert_eq!(s.host.focused_text_binding(), binding);
                assert_eq!(s.host.focus_epoch(), epoch);
                assert_eq!(s.host.semantic_mount(COMPOSER), mount);
                assert_eq!(s.host.focused(), Some(COMPOSER));
                assert_eq!(
                    keys.iter()
                        .map(|id| s.host.semantic_mount(*id))
                        .collect::<Vec<_>>(),
                    mounts
                );
                assert_eq!(
                    s.host
                        .get(SCROLLER)
                        .unwrap()
                        .component::<ScrollContent>()
                        .unwrap()
                        .reading_document_version(),
                    Some(&version)
                );
                s.host.event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true,
                });
                assert!(s.host.drain_actions().is_empty());
            }
        }
    }
}

#[test]
fn rejected_notice_structure_does_not_cancel_neighbor_composer() {
    let (mut s, _) = fixture::build().unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(
                NOTICE,
                ToastContent::new(
                    "Toast",
                    Toast::new("Saved", NoticeTone::Info, 5000),
                    DISMISS,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    s.host
        .reorder(ROOT, &[SCROLLER, MESSAGE, NOTICE, LATEST, COMPOSER])
        .unwrap();
    let mut t = coretext::Text::new(1.);
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    s.host
        .get_mut(NOTICE)
        .unwrap()
        .component_mut::<ToastContent>()
        .unwrap()
        .children_mut()
        .unwrap()
        .clear();
    assert!(
        s.host
            .show_toast(NOTICE, "New", NoticeTone::Error, 5000)
            .is_err()
    );
    let f = Rect::new(0., 0., 360., 1000.);
    assert!(
        s.host
            .render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
            .is_err()
    );
    assert_eq!(s.host.focused_text_binding(), binding);
    s.host
        .event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
            0., 0.,
        )));
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert_eq!(s.host.focused_text_binding(), binding);
    assert_eq!(s.host.focus_epoch(), epoch);
    let after = s.host.text_input_snapshot().unwrap();
    assert_eq!(after.text, before.text);
    assert_eq!(after.marked, before.marked);
    assert_eq!(after.selection, before.selection);
    assert!(s.host.drain_actions().is_empty());
}
