#![cfg(target_os = "macos")]
#[path = "../src/reading_checkpoint.rs"]
mod consumer;
use ContentReadingCheckpointError as E;
use consumer::Reader;
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ReadingUpdateError, ScrollRevealOptions,
    ScrollRevealPlacement, TextSurfaceHost, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    scale: f32,
    shaper: CoreTextShaper,
}
impl Text {
    fn new(scale: f32) -> Self {
        Self {
            scale,
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, w: f32) -> Size {
        let p = self.shape_surface(s, role, Some(w));
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
impl TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        self.scale
    }
    fn shape_surface(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        w: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(role.style().size * self.scale),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.0,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn paint(r: &mut Reader, width: f32, scale: f32) {
    let frame = Rect::new(12., 12., width, 420.);
    r.host
        .render(
            &mut Scene::new(),
            &mut Text::new(scale),
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
}
fn wheel(r: &mut Reader, dy: f32) {
    let at = r.host.get(r.owner).unwrap().frame().center();
    assert!(
        r.host
            .event(&ContentEvent::Scroll {
                position: at,
                delta: Point::new(0., dy),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 1.
            })
            .consumed
    );
}
fn mid() -> Reader {
    let mut r = Reader::new(1000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    r.host
        .reveal_scroll_reading_item(
            r.owner,
            &r.version,
            r.id("entry-10"),
            ScrollRevealOptions {
                placement: ScrollRevealPlacement::Start(
                    tekes_ui_controls::extended::Axis::Vertical,
                ),
                animated: false,
                ..Default::default()
            },
            &mut Text::new(1.),
        )
        .unwrap();
    paint(&mut r, 420., 1.);
    wheel(&mut r, 37.25);
    paint(&mut r, 420., 1.);
    r
}
fn history(c: &ContentReadingCheckpoint) -> (&str, f32, f32) {
    match &c.position {
        ContentReadingPosition::History {
            item,
            within,
            screen_y,
            ..
        } => (item, *within, *screen_y),
        _ => panic!("expected history"),
    }
}
fn roundtrip(c: &ContentReadingCheckpoint) -> ContentReadingCheckpoint {
    // Storage is deliberately a consumer concern. Unknown/missing/syntax error
    // must propagate from decoding instead of becoming FollowingLatest.
    let position = match &c.position {
        ContentReadingPosition::FollowingLatest => serde_json::json!({"kind":"following"}),
        ContentReadingPosition::History {
            item,
            within,
            screen_y,
            fallback,
        } => {
            serde_json::json!({"kind":"history", "item":item,"within":within,"screen_y":screen_y,"fallback":fallback})
        }
    };
    let json = serde_json::to_vec(&serde_json::json!({"format":c.format,"document":c.document,"revision":c.revision,"position":position})).unwrap();
    decode(&json).unwrap()
}
fn decode(bytes: &[u8]) -> Result<ContentReadingCheckpoint, String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let invalid = || "invalid checkpoint storage".to_string();
    let p = &v["position"];
    Ok(ContentReadingCheckpoint {
        format: v["format"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or_else(invalid)?,
        document: v["document"].as_str().ok_or_else(invalid)?.into(),
        revision: v["revision"].as_u64().ok_or_else(invalid)?,
        position: match p["kind"].as_str().ok_or_else(invalid)? {
            "following" => ContentReadingPosition::FollowingLatest,
            "history" => ContentReadingPosition::History {
                item: p["item"].as_str().ok_or_else(invalid)?.into(),
                within: p["within"].as_f64().ok_or_else(invalid)? as f32,
                screen_y: p["screen_y"].as_f64().ok_or_else(invalid)? as f32,
                fallback: p["fallback"]
                    .as_array()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(|v| v.as_str().map(String::from).ok_or_else(invalid))
                    .collect::<Result<_, _>>()?,
            },
            _ => return Err("unsupported storage kind".into()),
        },
    })
}
fn read_saved(
    read: Result<Option<Vec<u8>>, String>,
) -> Result<Option<ContentReadingCheckpoint>, String> {
    read?.map(|bytes| decode(&bytes)).transpose()
}

fn editor_baseline(r: &mut Reader) -> tekes_ui_controls::TextEditBaseline {
    r.host.focus(Some(r.editor), true);
    r.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    r.host
        .get(r.editor)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .external_baseline()
}

#[test]
fn persisted_mixed_history_survives_destruction_new_ids_and_coretext_reflow() {
    let old = mid();
    let c = roundtrip(&old.capture().unwrap());
    let (key, within, screen_y) = history(&c);
    assert!(within > 0.0);
    let old_id = old.id(key);
    let old_y = old.host.get(old_id).unwrap().frame().min_y();
    drop(old);
    let mut new = Reader::new(8000, 2, &[], &[]);
    assert_ne!(new.id(key), old_id);
    let request = new.request();
    paint(&mut new, 420., 1.);
    let baseline = editor_baseline(&mut new);
    new.restore(&request, &c).unwrap();
    paint(&mut new, 420., 1.);
    assert_eq!(
        new.scroll().reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Preserved(new.id(key))
    );
    let restored_y = new.host.get(new.id(key)).unwrap().frame().min_y();
    eprintln!(
        "checkpoint fixed-width key={key} old_id={old_id:?} new_id={:?} within={within} old_y={old_y} restored_y={restored_y} error={}",
        new.id(key),
        (restored_y - old_y).abs()
    );
    assert!((restored_y - old_y).abs() <= 1.0);
    assert_eq!(new.host.focused(), Some(new.editor));
    assert_eq!(
        new.host
            .get(new.editor)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .external_baseline(),
        baseline
    );
    assert_eq!(new.restore(&request, &c), Err(E::Superseded));
    for (width, scale) in [(260., 1.), (260., 1.6), (640., 0.8)] {
        paint(&mut new, width, scale);
        let current = new.capture().unwrap();
        let (k, local, y) = history(&current);
        assert_eq!(k, key);
        assert!(
            (local - within).abs() <= 1.0,
            "item-local point moved: {local} vs {within}"
        );
        assert!((y - screen_y).abs() <= 1.0);
        eprintln!(
            "checkpoint reflow width={width} scale={scale} key={k} within={local} screen_y={y}"
        );
        assert_eq!(
            new.host
                .get(new.editor)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .external_baseline(),
            baseline
        );
    }
}

#[test]
fn following_and_history_restore_without_implicit_follow_on_append() {
    let mut old = mid();
    let history = old.capture().unwrap();
    old.host
        .get_mut(old.owner)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut old, 420., 1.);
    let latest = roundtrip(&old.capture().unwrap());
    assert!(matches!(
        latest.position,
        ContentReadingPosition::FollowingLatest
    ));
    drop(old);
    for c in [&history, &latest] {
        let mut r = Reader::new(4000, 1, &[], &[]);
        paint(&mut r, 420., 1.);
        r.restore(&r.request(), c).unwrap();
        paint(&mut r, 420., 1.);
        let following = matches!(c.position, ContentReadingPosition::FollowingLatest);
        assert_eq!(r.scroll().is_following_latest(), following);
        let before = r.scroll().scroll.offset.y;
        let extra = ComponentId(9000);
        let mut ids: Vec<_> = r.keys.iter().map(|(_, id)| *id).collect();
        ids.push(extra);
        let (version, ()) = r
            .host
            .update_scroll_reading_document(r.owner, &r.version, 2, ids, |h| {
                h.insert(
                    r.list,
                    Content::new(extra, TextContent::new("New appended content. ".repeat(80))),
                )
                .unwrap();
            })
            .unwrap();
        r.version = version;
        r.keys.push(("new".into(), extra));
        paint(&mut r, 420., 1.);
        assert_eq!(r.scroll().is_following_latest(), following);
        if following {
            assert!(r.scroll().scroll.offset.y > before);
        } else {
            assert!((r.scroll().scroll.offset.y - before).abs() <= 1.0);
        }
        r.host
            .get_mut(r.owner)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .follow_latest();
        paint(&mut r, 420., 1.);
        assert!(r.scroll().is_following_latest());
    }
}

#[test]
fn unloaded_target_is_unavailable_deleted_target_uses_saved_successor() {
    let old = mid();
    let c = old.capture().unwrap();
    let n = history(&c)
        .0
        .strip_prefix("entry-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    drop(old);
    let mut unloaded = Reader::new(6000, 1, &[], &[n]);
    paint(&mut unloaded, 420., 1.);
    let before = unloaded.capture().unwrap();
    assert_eq!(
        unloaded.restore(&unloaded.request(), &c),
        Err(E::ItemUnavailable(format!("entry-{n}")))
    );
    assert_eq!(unloaded.capture().unwrap(), before);
    let mut deleted = Reader::new(7000, 2, &[n], &[]);
    paint(&mut deleted, 420., 1.);
    deleted.restore(&deleted.request(), &c).unwrap();
    paint(&mut deleted, 420., 1.);
    assert_eq!(
        deleted.scroll().reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(Some(deleted.id(&format!("entry-{}", n + 1))))
    );
}

#[test]
fn stale_attachment_revision_and_new_user_intent_reject_without_partial_install() {
    let c = mid().capture().unwrap();
    let mut r = Reader::new(6000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    let baseline = editor_baseline(&mut r);
    let request = r.request();
    wheel(&mut r, 150.);
    paint(&mut r, 420., 1.);
    let before = r.capture().unwrap();
    assert_eq!(r.restore(&request, &c), Err(E::Superseded));
    assert_eq!(r.capture().unwrap(), before);
    let stale = r.request();
    let ids = r.keys.iter().map(|(_, id)| *id).collect();
    let (v, ()) = r
        .host
        .set_scroll_reading_document(r.owner, "B", 1, ids, |_| ())
        .unwrap();
    let ids = r.keys.iter().map(|(_, id)| *id).collect();
    r.version = r
        .host
        .set_scroll_reading_document(r.owner, "business-document", 1, ids, |_| ())
        .unwrap()
        .0;
    assert_ne!(r.version.attachment(), v.attachment());
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.restore(&stale, &c),
        Err(E::Reading(ReadingUpdateError::WrongDocument))
    );
    let request = r.request();
    let ids = r.keys.iter().map(|(_, id)| *id).collect();
    r.version = r
        .host
        .update_scroll_reading_document(r.owner, &r.version, 2, ids, |_| ())
        .unwrap()
        .0;
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.restore(&request, &c),
        Err(E::Reading(ReadingUpdateError::StaleRevision))
    );
    assert_eq!(r.host.focused(), Some(r.editor));
    assert_eq!(
        r.host
            .get(r.editor)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .external_baseline(),
        baseline
    );
}

#[test]
fn user_scroll_after_install_before_layout_cancels_pending_checkpoint() {
    let c = mid().capture().unwrap();
    let mut r = Reader::new(4000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    r.restore(&r.request(), &c).unwrap();
    wheel(&mut r, 80.);
    let offset = r.scroll().scroll.offset.y;
    paint(&mut r, 420., 1.);
    assert!((r.scroll().scroll.offset.y - offset).abs() <= 1.0);
    assert!(r.scroll().reading_result().unwrap().user_input_won);
    assert_ne!(history(&r.capture().unwrap()).0, history(&c).0);
}

#[test]
fn corrupt_values_wrong_document_and_bad_mapping_are_atomic_distinct_errors() {
    let c = mid().capture().unwrap();
    let mut r = Reader::new(4000, 1, &[], &[]);
    assert_eq!(r.capture(), Err(E::UnlaidOut));
    let req = r.request();
    assert_eq!(r.restore(&req, &c), Err(E::UnlaidOut));
    paint(&mut r, 420., 1.);
    let before = r.capture().unwrap();
    let mut bad = c.clone();
    bad.format = 99;
    assert_eq!(r.restore(&req, &bad), Err(E::UnsupportedFormat));
    bad = c.clone();
    bad.document = "other".into();
    assert_eq!(
        r.restore(&req, &bad),
        Err(E::Reading(ReadingUpdateError::WrongDocument))
    );
    bad = c.clone();
    bad.revision = 9;
    assert_eq!(
        r.restore(&req, &bad),
        Err(E::Reading(ReadingUpdateError::StaleRevision))
    );
    for x in [f32::NAN, f32::INFINITY, -1.0] {
        bad = c.clone();
        if let ContentReadingPosition::History { within, .. } = &mut bad.position {
            *within = x;
        }
        assert_eq!(r.restore(&req, &bad), Err(E::InvalidCheckpoint));
    }
    bad = c.clone();
    if let ContentReadingPosition::History { item, fallback, .. } = &mut bad.position {
        fallback.push(item.clone());
    }
    assert_eq!(r.restore(&req, &bad), Err(E::InvalidCheckpoint));
    let duplicate = vec![r.keys[0].clone(); r.keys.len()];
    assert_eq!(
        r.host
            .restore_scroll_reading_checkpoint(&req, &c, &duplicate),
        Err(E::InvalidItemKeys)
    );
    assert_eq!(r.capture().unwrap(), before);
    // Failures did not consume the original request or replace the token.
    r.restore(&req, &c).unwrap();
    assert_eq!(r.scroll().reading_document_version(), Some(&r.version));
}

#[test]
fn pagination_can_retry_original_request_after_mounting_declared_item() {
    let old = mid();
    let c = old.capture().unwrap();
    let n = history(&c)
        .0
        .strip_prefix("entry-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    drop(old);
    let mut r = Reader::new(5000, 1, &[], &[n]);
    let req = r.request();
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.restore(&req, &c),
        Err(E::ItemUnavailable(format!("entry-{n}")))
    );
    // Model revision already declared this item. Materializing its view does
    // not substitute the original request or grant a new input generation.
    let id = r.id(&format!("entry-{n}"));
    r.host
        .insert(
            r.list,
            Content::new(id, TextContent::new("Loaded history. ".repeat(100))),
        )
        .unwrap();
    let ids: Vec<_> = r.keys.iter().map(|(_, id)| *id).collect();
    r.host.reorder(r.list, &ids).unwrap();
    paint(&mut r, 420., 1.);
    r.restore(&req, &c).unwrap();
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.scroll().reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Preserved(id)
    );
    assert!(r.host.get(id).unwrap().visible().is_some());
}

#[test]
fn explicit_follow_stop_navigation_and_same_key_reattachment_retire_requests() {
    let c = mid().capture().unwrap();
    let mut r = Reader::new(5000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    let req = r.request();
    r.host
        .get_mut(r.owner)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut r, 420., 1.);
    assert_eq!(r.restore(&req, &c), Err(E::Superseded));
    let req = r.request();
    r.host
        .get_mut(r.owner)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .stop_following();
    assert_eq!(r.restore(&req, &c), Err(E::Superseded));
    // get_mut retires the Host's navigation geometry lease. Complete normal
    // rendering before issuing the next explicit navigation command.
    paint(&mut r, 420., 1.);
    let req = r.request();
    r.host
        .reveal_scroll_reading_item(
            r.owner,
            &r.version,
            r.id("entry-8"),
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &mut Text::new(1.),
        )
        .unwrap();
    paint(&mut r, 420., 1.);
    assert_eq!(r.restore(&req, &c), Err(E::Superseded));
    let req = r.request();
    let ids = r.keys.iter().map(|(_, id)| *id).collect();
    r.version = r
        .host
        .set_scroll_reading_document(r.owner, "business-document", 1, ids, |_| ())
        .unwrap()
        .0;
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.restore(&req, &c),
        Err(E::Reading(ReadingUpdateError::WrongDocument))
    );
}

#[test]
fn shortened_item_clamps_and_deleted_neighbors_fall_back_to_first_loaded_item() {
    let c = mid().capture().unwrap();
    let mut r = Reader::new(5000, 2, &[], &[]);
    paint(&mut r, 420., 1.);
    let id = r.id(history(&c).0);
    // Simulate a newer document shrinking the item below its saved local point.
    let mut short = c.clone();
    if let ContentReadingPosition::History { within, .. } = &mut short.position {
        *within = r.host.get(id).unwrap().frame().size.height + 500.0;
    }
    r.restore(&r.request(), &short).unwrap();
    paint(&mut r, 420., 1.);
    assert!(r.scroll().reading_result().unwrap().clamped);
    let mut removed = c;
    if let ContentReadingPosition::History { item, fallback, .. } = &mut removed.position {
        *item = "removed".into();
        fallback.clear();
    }
    r.restore(&r.request(), &removed).unwrap();
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.scroll().reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(Some(r.id("entry-0")))
    );
}

#[test]
fn consumer_preserves_absent_storage_failure_and_corrupt_storage_as_distinct_states() {
    assert_eq!(read_saved(Ok(None)), Ok(None));
    assert_eq!(
        read_saved(Err("disk unavailable".into())),
        Err("disk unavailable".into())
    );
    assert!(read_saved(Ok(Some(b"broken".to_vec()))).is_err());
    assert!(
        read_saved(Ok(Some(
            br#"{"format":1,"document":"doc","revision":1,"position":{"kind":"unknown"}}"#.to_vec()
        )))
        .is_err()
    );
}

#[test]
fn fresh_downward_input_cancels_imported_follow_before_first_restore_layout() {
    let mut c = mid().capture().unwrap();
    c.position = ContentReadingPosition::FollowingLatest;
    let mut r = Reader::new(5000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    r.restore(&r.request(), &c).unwrap();
    wheel(&mut r, 80.0);
    let offset = r.scroll().scroll.offset.y;
    paint(&mut r, 420., 1.);
    assert!(!r.scroll().is_following_latest());
    assert!((r.scroll().scroll.offset.y - offset).abs() <= 1.0);
    assert!(r.scroll().reading_result().unwrap().user_input_won);
}

#[test]
fn deleted_anchor_skips_unloaded_neighbors_in_existing_layout_fallback_order() {
    let c = mid().capture().unwrap();
    let n = history(&c)
        .0
        .strip_prefix("entry-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let mut r = Reader::new(5000, 2, &[n], &[n + 1]);
    paint(&mut r, 420., 1.);
    r.restore(&r.request(), &c).unwrap();
    paint(&mut r, 420., 1.);
    assert_eq!(
        r.scroll().reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::Fallback(Some(r.id(&format!("entry-{}", n + 2))))
    );
}

#[test]
fn failed_layout_cannot_export_or_install_a_checkpoint_from_stale_frames() {
    struct MissingText;
    impl ButtonTextRenderer for MissingText {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(1.0, 1.0)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    let c = mid().capture().unwrap();
    let mut r = Reader::new(5000, 1, &[], &[]);
    paint(&mut r, 420., 1.);
    let request = r.request();
    let frame = Rect::new(12.0, 12.0, 420.0, 420.0);
    assert!(
        r.host
            .render(
                &mut Scene::new(),
                &mut MissingText,
                frame,
                frame,
                Theme::LIGHT.into()
            )
            .is_err()
    );
    assert_eq!(r.capture(), Err(E::UnlaidOut));
    assert_eq!(r.restore(&request, &c), Err(E::UnlaidOut));
    paint(&mut r, 420., 1.);
    r.restore(&request, &c).unwrap();
}
