#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::{
    empty_state_content::{self as fixture, MESSAGE},
    scroll_updates::{COMPOSER, SCROLLER},
};
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::surface_names};
fn paint(h: &mut ContentHost, t: &mut coretext::Text, p: Palette) {
    t.services = true;
    t.atlas.begin_frame();
    let f = Rect::new(0.25, 0.5, 360., 720.);
    h.render(&mut Scene::new(), t, f, f, p).unwrap();
}
#[test]
fn original_reading_anchor_and_composer_survive_action_height_transactions() {
    let (mut s, mut version) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    s.host
        .get_mut(SCROLLER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let pos = s.host.get(SCROLLER).unwrap().frame().center();
    assert!(
        s.host
            .event(&ContentEvent::Scroll {
                position: pos,
                delta: Point::new(0., -80.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 1.
            })
            .consumed
    );
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let keys = s.item_keys();
    let anchor = *keys
        .iter()
        .find(|id| s.host.get(**id).unwrap().visible().is_some())
        .unwrap();
    let y = s.host.get(anchor).unwrap().frame().min_y();
    let mount = s.host.semantic_mount(anchor);
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let editor = s.host.semantic_mount(COMPOSER);
    s.host.drain_actions();
    for theme in Theme::ALL {
        for surface in surface_names() {
            for action in [None, Some("Open 文档"), Some("Create")] {
                let (v, ()) = s
                    .host
                    .update_scroll_reading_document(
                        SCROLLER,
                        &version,
                        version.revision() + 1,
                        keys.clone(),
                        |h| {
                            h.replace_empty_state(MESSAGE, fixture::model(action))
                                .unwrap();
                            h.set_inherited_surface(surface).unwrap();
                            let f = Rect::new(0., 0., 360., 720.);
                            assert_eq!(
                                h.render(
                                    &mut Scene::new(),
                                    &mut coretext::Text::new(1.),
                                    f,
                                    f,
                                    Theme::LIGHT.into()
                                ),
                                Err(CompositionError::ReadingTransactionInProgress)
                            );
                        },
                    )
                    .unwrap();
                version = v;
                paint(&mut s.host, &mut t, Palette::new(theme, 2).unwrap());
                let scroll = s
                    .host
                    .get(SCROLLER)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap();
                assert_eq!(scroll.reading_document_version(), Some(&version));
                assert_eq!(scroll.reading_items(), keys.as_slice());
                let result = scroll.reading_result().unwrap();
                assert_eq!(result.version, version);
                assert_eq!(result.anchor, ContentReadingAnchorUpdate::Preserved(anchor));
                assert!(!result.user_input_won);
                assert!((s.host.get(anchor).unwrap().frame().min_y() - y).abs() < 1.);
                assert_eq!(s.host.semantic_mount(anchor), mount);
                let after = s.host.text_input_snapshot().unwrap();
                assert_eq!(after.text, before.text);
                assert_eq!(after.marked, before.marked);
                assert_eq!(after.selection, before.selection);
                assert_eq!(after.binding, before.binding);
                assert_eq!(s.host.focused_text_binding(), binding);
                assert_eq!(s.host.semantic_mount(COMPOSER), editor);
                assert_eq!(s.host.focused(), Some(COMPOSER));
                s.host.event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true,
                });
                assert!(s.host.drain_actions().is_empty());
            }
        }
    }
}
