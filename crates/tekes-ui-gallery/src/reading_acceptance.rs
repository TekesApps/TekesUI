//! Opt-in native Gallery consumer of the independent public reading fixture.
use crate::gallery::text::TextEngine;
use crate::{reading_key as native_key, reading_pointer as native_pointer};
use tekes_ui_composition_consumer::reading_prepend::ReadingFixture;
use tekes_ui_controls::composition::ContentEvent as C;
use tekes_ui_platform_macos::{
    App, AppConfig, Event, MenuBar, ScrollPhase, Window, WindowConfig, WindowHandler,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Spacing, Theme};

pub(crate) fn try_run() -> bool {
    let Some(out) = std::env::var_os("TEKESUI_READING_PREPEND_AUDIT_OUT") else {
        return false;
    };
    App::new(AppConfig {
        name: Some("TekesUI Gallery 阅读锚点".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI Gallery · 历史前插".into(),
            size: tekes_ui_platform_macos::Size {
                width: 800.,
                height: 720.,
            },
            ..Default::default()
        },
        MenuBar::default(),
        Host {
            fixture: {
                let mut fixture = ReadingFixture::new();
                if std::env::var_os("TEKESUI_TEXT_EDIT_AUDIT").is_some() {
                    use tekes_ui_controls::{
                        TextSurface,
                        composition::{TextFieldContent, TextSubmitPolicy, TextTabPolicy},
                    };
                    let field = fixture
                        .controls
                        .get_mut(tekes_ui_composition_consumer::reading_prepend::EDITOR)
                        .unwrap()
                        .component_mut::<TextFieldContent>()
                        .unwrap();
                    field.surface = TextSurface::new(&field.surface.input.text, true, false);
                    field.tab_policy = TextTabPolicy::Insert;
                    field.submit_policy = TextSubmitPolicy::CommandEnter;
                }
                fixture
            },
            text: TextEngine::new(),
            scene: Scene::new(),
            renderer: None,
            theme: Theme::LIGHT,
            start: std::time::Instant::now(),
            reduced: false,
            active: true,
            out: out.into(),
            expected: None,
            reports: Vec::new(),
            pointer_reports: Vec::new(),
            text_key_reports: Vec::new(),
        },
    );
    true
}
struct Host {
    fixture: ReadingFixture,
    text: TextEngine,
    scene: Scene,
    renderer: Option<Renderer>,
    theme: Theme,
    start: std::time::Instant,
    reduced: bool,
    active: bool,
    out: std::path::PathBuf,
    expected: Option<(usize, f32)>,
    reports: Vec<serde_json::Value>,
    pointer_reports: Vec<serde_json::Value>,
    text_key_reports: Vec<serde_json::Value>,
}
impl Host {
    fn paint(&mut self, window: &Window) {
        self.text.set_scale(window.scale() as f32);
        self.text.set_locale(crate::gallery::locale::Locale::ZhCn);
        self.text.set_motion_enabled(true);
        self.text
            .set_motion_context(self.start.elapsed(), self.reduced);
        self.text.begin_frame();
        self.scene.clear();
        let size = window.size();
        let frame = Rect::new(0., 0., size.width as f32, size.height as f32);
        self.fixture.paint(
            &mut self.text,
            &mut self.scene,
            frame.inset(Spacing::Loose.value()),
            frame,
            Palette::from(self.theme),
        );
        if let Some((index, before)) = self.expected.take() {
            let after = self.fixture.surface.caret_doc(index).min_y()
                - self.fixture.surface.scroll.offset.y;
            self.reports.push(serde_json::json!({"pages":self.fixture.pages,"utf16":index,"before_y":before,"after_y":after,"error_pt":(before-after).abs()}));
        }
        if let Some(renderer) = &mut self.renderer {
            renderer
                .render(
                    &self.scene,
                    self.text.atlas_mut(),
                    Palette::from(self.theme).color("surface-1").unwrap(),
                )
                .unwrap();
        }
        window.set_title(&format!(
            "TekesUI Gallery · 历史前插 {} 页",
            self.fixture.pages
        ));
        window.set_animating(self.text.motion_frame_requested());
        self.publish(window);
        self.save();
    }
    fn publish(&self, window: &Window) {
        window.set_text_input_state(
            self.fixture
                .controls
                .focused_text()
                .filter(|_| self.active)
                .map(|s| {
                    let caret = s.caret();
                    let origin = s.origin();
                    tekes_ui_platform_macos::TextInputState {
                        text: s.input.text.clone(),
                        selection: s.input.selection(),
                        marked: s.input.marked.clone(),
                        caret_index: s.input.caret,
                        caret_origin: tekes_ui_platform_macos::Point {
                            x: caret.min_x() as f64,
                            y: caret.min_y() as f64,
                        },
                        caret_size: tekes_ui_platform_macos::Size {
                            width: caret.size.width as f64,
                            height: caret.size.height as f64,
                        },
                        read_only: s.input.read_only,
                        carets: s
                            .paragraph
                            .as_ref()
                            .map(|p| {
                                p.lines
                                    .iter()
                                    .filter(|l| {
                                        origin.y + (l.top + l.height) as f32 >= s.inner.min_y()
                                            && origin.y + l.top as f32 <= s.inner.max_y()
                                    })
                                    .flat_map(|l| {
                                        l.carets.iter().map(move |(i, x)| {
                                            (
                                                *i,
                                                tekes_ui_platform_macos::Point {
                                                    x: f64::from(origin.x) + x,
                                                    y: f64::from(origin.y) + l.top,
                                                },
                                                tekes_ui_platform_macos::Size {
                                                    width: 1.,
                                                    height: l.height,
                                                },
                                            )
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }
                }),
        );
    }
    fn input(&mut self, event: Event) -> bool {
        let event = match event {
            Event::Mouse(m) => native_pointer::translate(m),
            Event::Scroll(s) => {
                let p = Point::new(s.position.x as f32, s.position.y as f32);
                if self.fixture.surface.contains_visible(p) {
                    let delta = Point::new(s.delta.x as f32, s.delta.y as f32);
                    self.fixture.reading.observe_scroll(
                        &mut self.fixture.surface,
                        delta,
                        s.modifiers.shift,
                    );
                    return self
                        .fixture
                        .surface
                        .wheel_at(
                            p,
                            delta,
                            s.precise,
                            matches!(s.phase, ScrollPhase::Began | ScrollPhase::MayBegin),
                            matches!(s.phase, ScrollPhase::Ended | ScrollPhase::Cancelled),
                            s.momentum_phase != ScrollPhase::None,
                            s.timestamp,
                            s.modifiers.shift,
                        )
                        .unwrap_or(false);
                }
                return false;
            }
            Event::TextCommit { text, replacement } => C::Commit { text, replacement },
            Event::TextPreedit {
                text,
                selection,
                clauses,
                replacement,
            } => C::Preedit {
                text,
                selection,
                clauses,
                replacement,
            },
            Event::TextUnmark => C::Unmark,
            Event::TextCommand(c) => C::TextCommand(c),
            Event::Key(k) => C::KeyInput(native_key::key(k)),
            _ => return false,
        };
        let before = self
            .fixture
            .controls
            .text_surface(tekes_ui_composition_consumer::reading_prepend::EDITOR)
            .map(|s| s.input.selection());
        let changed = self.fixture.controls.event(&event).changed;
        if let C::Pointer(p) = &event
            && self.pointer_reports.len() < 512
        {
            self.pointer_reports.push(serde_json::json!({"phase":format!("{:?}",p.phase),"button":format!("{:?}",p.button),"click_count":p.click_count,"shift":p.modifiers.shift,"position":[p.position.x,p.position.y],"before_selection":before,"after_selection":self.fixture.controls.text_surface(tekes_ui_composition_consumer::reading_prepend::EDITOR).map(|s|s.input.selection()),"focus":self.fixture.controls.focused().map(|id|id.0)}));
            self.save();
        }
        changed
    }
    fn save(&self) {
        std::fs::create_dir_all(&self.out).unwrap();
        std::fs::write(self.out.join("reading-events.json"),serde_json::to_vec_pretty(&serde_json::json!({"pages":self.fixture.pages,"reports":self.reports,"pointer_reports":self.pointer_reports,"text_key_reports":self.text_key_reports,"editor_selection":self.fixture.controls.text_surface(tekes_ui_composition_consumer::reading_prepend::EDITOR).map(|s|s.input.selection()),"following":self.fixture.reading.is_following_latest(),"version":self.fixture.reading.document_version().map(|v|v.revision()),"draft":self.fixture.controls.text_surface(tekes_ui_composition_consumer::reading_prepend::EDITOR).map(|s|&s.input.text),"focus":self.fixture.controls.focused().map(|v|v.0),"scope":"native Gallery paging and real layout; not physical IME/VoiceOver/performance acceptance"})).unwrap()).unwrap();
    }
}
impl WindowHandler for Host {
    fn handle(&mut self, event: Event, window: &mut Window) {
        match event {
            Event::TextKey(report) => {
                let response = self
                    .fixture
                    .controls
                    .event(&native_key::translate(report.clone()));
                self.publish(window);
                if response.changed {
                    window.request_redraw();
                }
                if self.text_key_reports.len() < 512 {
                    self.text_key_reports.push(serde_json::json!({
                        "event":format!("{report:?}"),
                        "outcome":format!("{:?}",response.text_key),
                        "submit_intent":response.activated,
                        "draft":self.fixture.controls.text_surface(tekes_ui_composition_consumer::reading_prepend::EDITOR).map(|s|&s.input.text),
                        "focus":self.fixture.controls.focused().map(|id|id.0),
                    }));
                }
                self.save();
            }
            Event::Created => {
                assert!(window.set_text_key_events(true));
                // SAFETY: this Gallery handler owns the renderer until the
                // window run returns; the existing view retains the layer.
                self.renderer = Some(
                    unsafe {
                        Renderer::new_for_metal_layer(
                            window.metal_layer(),
                            window.drawable_size(),
                            window.scale() as f32,
                        )
                    }
                    .unwrap(),
                );
                if std::env::var_os("TEKESUI_READING_PREPEND_BACKGROUND").is_some() {
                    let wake = window.wake_proxy();
                    std::thread::spawn(move || {
                        // One finite arrival, not a data/frame polling loop.
                        std::thread::sleep(std::time::Duration::from_secs(10));
                        wake.wake();
                    });
                }
            }
            Event::Wake => {
                let before = self.fixture.surface.reflow_anchor.map(|a| {
                    (
                        a.utf16,
                        self.fixture.surface.caret_doc(a.utf16).min_y()
                            - self.fixture.surface.scroll.offset.y,
                    )
                });
                let len = self.fixture.surface.input.len();
                self.fixture.prepend().unwrap();
                self.expected =
                    before.map(|(i, y)| (i + self.fixture.surface.input.len() - len, y));
                window.request_redraw();
            }
            Event::Resized {
                drawable_size,
                scale,
                ..
            } => {
                if let Some(r) = &mut self.renderer {
                    r.resize(drawable_size, scale as f32);
                }
            }
            Event::Redraw | Event::AnimationFrame(_) => self.paint(window),
            Event::CloseRequested => window.close(),
            Event::SystemMotionChanged(v) => {
                self.reduced = v;
                window.request_redraw();
            }
            Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.theme = Theme {
                    appearance: if dark {
                        tekes_ui_tokens::Appearance::Dark
                    } else {
                        tekes_ui_tokens::Appearance::Light
                    },
                    contrast: if increase_contrast {
                        tekes_ui_tokens::Contrast::Increased
                    } else {
                        tekes_ui_tokens::Contrast::Standard
                    },
                };
                window.request_redraw();
            }
            Event::Focus(active) => {
                self.active = active;
                if !active {
                    self.fixture.controls.event(&C::Cancel);
                    self.fixture.surface.cancel_interaction();
                }
                self.publish(window);
                window.request_redraw();
            }
            event => {
                let before = self.fixture.surface.reflow_anchor.map(|a| {
                    (
                        a.utf16,
                        self.fixture.surface.caret_doc(a.utf16).min_y()
                            - self.fixture.surface.scroll.offset.y,
                    )
                });
                let len = self.fixture.surface.input.len();
                let pages = self.fixture.pages;
                let changed = self.input(event);
                let action = self.fixture.process_actions().unwrap();
                if self.fixture.pages != pages {
                    self.expected =
                        before.map(|(i, y)| (i + self.fixture.surface.input.len() - len, y));
                }
                self.publish(window);
                if changed || action {
                    window.request_redraw();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_prepend_fixture_keeps_real_gpu_reading_position_in_four_themes() {
        let mut renderer = Renderer::new_headless().expect("real GPU required");
        let out = std::env::var_os("TEKESUI_PREPEND_GPU_OUT").map(std::path::PathBuf::from);
        if let Some(p) = &out {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut reports = Vec::new();
        for (theme_id, theme) in Theme::ALL.into_iter().enumerate() {
            for width in [360u32, 800] {
                for scale in [1u32, 2] {
                    let mut fixture = ReadingFixture::new();
                    let mut text = TextEngine::new();
                    text.set_locale(crate::gallery::locale::Locale::ZhCn);
                    text.set_scale(scale as f32);
                    text.set_type_scale(if width == 360 { 1.5 } else { 1. });
                    let frame = Rect::new(0., 0., width as f32, 640.);
                    let mut anchor: Option<(usize, f32)> = None;
                    for stage in 0..3 {
                        if stage > 0 {
                            let len = fixture.surface.input.len();
                            fixture.prepend().unwrap();
                            let delta = fixture.surface.input.len() - len;
                            if let Some((i, y)) = anchor {
                                anchor = Some((i + delta, y));
                            }
                        }
                        text.begin_frame();
                        let mut scene = Scene::new();
                        fixture.paint(
                            &mut text,
                            &mut scene,
                            frame.inset(Spacing::Loose.value()),
                            frame,
                            theme.into(),
                        );
                        let a = fixture.surface.reflow_anchor.unwrap();
                        let y = fixture.surface.caret_doc(a.utf16).min_y()
                            - fixture.surface.scroll.offset.y;
                        if let Some((i, old_y)) = anchor {
                            assert_eq!(a.utf16, i);
                            assert!((y - old_y).abs() <= 1.);
                            reports.push(serde_json::json!({"theme":theme_id,"width":width,"scale":scale,"stage":stage,"error_pt":(y-old_y).abs()}));
                        } else {
                            anchor = Some((a.utf16, y));
                        }
                        assert!(
                            fixture
                                .controls
                                .get(tekes_ui_composition_consumer::reading_prepend::EDITOR)
                                .unwrap()
                                .frame()
                                .max_y()
                                <= frame.max_y()
                        );
                        let rgba = renderer
                            .render_to_rgba8(
                                &scene,
                                text.atlas_mut(),
                                (width * scale, 640 * scale),
                                scale as f32,
                                Palette::from(theme).color("surface-1").unwrap(),
                            )
                            .unwrap();
                        if let Some(p) = &out {
                            let f = std::fs::File::create(p.join(format!(
                                "reading-{theme_id}-{width}@{scale}x-stage{stage}.png"
                            )))
                            .unwrap();
                            let mut enc = png::Encoder::new(f, width * scale, 640 * scale);
                            enc.set_color(png::ColorType::Rgba);
                            enc.set_depth(png::BitDepth::Eight);
                            enc.write_header().unwrap().write_image_data(&rgba).unwrap();
                        }
                    }
                }
            }
        }
        if let Some(p) = out {
            std::fs::write(
                p.join("geometry.json"),
                serde_json::to_vec_pretty(&reports).unwrap(),
            )
            .unwrap();
        }
    }
    #[test]
    fn full_pointer_shift_selection_uses_original_gpu_paint_in_four_themes() {
        use tekes_ui_composition_consumer::reading_prepend::EDITOR;
        use tekes_ui_controls::composition::{
            PointerButton, PointerEvent, PointerModifiers, PointerPhase,
        };
        let out = std::env::var_os("TEKESUI_POINTER_GPU_OUT").map(std::path::PathBuf::from);
        if let Some(p) = &out {
            std::fs::create_dir_all(p).unwrap();
        }
        let mut renderer = Renderer::new_headless().expect("real GPU required");
        for (theme_id, theme) in Theme::ALL.into_iter().enumerate() {
            for scale in [1u32, 2] {
                let mut fixture = ReadingFixture::new();
                let mut text = TextEngine::new();
                text.set_scale(scale as f32);
                let frame = Rect::new(0., 0., 800., 640.);
                let mut baseline: Option<Vec<u8>> = None;
                for stage in 0..2 {
                    if stage == 1 {
                        let field = fixture.controls.text_surface(EDITOR).unwrap();
                        let point = |index| {
                            let rect = field.caret_doc(index);
                            Point::new(
                                field.origin().x + rect.min_x() + 0.1,
                                field.caret().center().y,
                            )
                        };
                        let a = point(2);
                        let b = point(12);
                        for (phase, position, shift) in [
                            (PointerPhase::Down, a, false),
                            (PointerPhase::Up, a, false),
                            (PointerPhase::Down, b, true),
                            (PointerPhase::Up, b, true),
                        ] {
                            fixture.controls.event(&C::Pointer(PointerEvent {
                                position,
                                phase,
                                button: Some(PointerButton::Primary),
                                modifiers: PointerModifiers {
                                    shift,
                                    ..Default::default()
                                },
                                click_count: 1,
                            }));
                        }
                        assert_eq!(fixture.controls.focused(), Some(EDITOR));
                        assert!(
                            !fixture
                                .controls
                                .text_surface(EDITOR)
                                .unwrap()
                                .input
                                .selection()
                                .is_empty()
                        );
                    }
                    text.begin_frame();
                    let mut scene = Scene::new();
                    fixture.paint(&mut text, &mut scene, frame.inset(16.), frame, theme.into());
                    let bytes = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            (800 * scale, 640 * scale),
                            scale as f32,
                            Palette::from(theme).color("surface-1").unwrap(),
                        )
                        .unwrap();
                    if let Some(before) = &baseline {
                        assert_ne!(before, &bytes, "selection must reach real GPU painter");
                    } else {
                        baseline = Some(bytes.clone());
                    }
                    if let Some(p) = &out {
                        let f = std::fs::File::create(
                            p.join(format!("pointer-{theme_id}@{scale}x-stage{stage}.png")),
                        )
                        .unwrap();
                        let mut encoder = png::Encoder::new(f, 800 * scale, 640 * scale);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&bytes)
                            .unwrap();
                    }
                }
            }
        }
    }
}
