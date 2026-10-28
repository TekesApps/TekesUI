//! Full generated packets, original status model and one passive semantic owner.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, extended::StatusBadge,
    layout_view::IconRenderer, paint::ButtonVariant,
};
use tekes_ui_scene::{ImageId, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Rgba, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
#[derive(Default)]
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 20.)
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
#[derive(Default)]
struct Icons {
    ink: Vec<Rgba>,
}
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("checked resources required")
    }
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        tint: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.ink.push(tint);
        Ok(IconImage::new(ImageId(1), std::rc::Rc::new(())))
    }
}
fn paint(h: &mut ContentHost, icons: &mut Icons, p: Palette) {
    let f = Rect::new(0., 0., 100., 100.);
    icons.ink.clear();
    h.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(&mut Text, icons),
        f,
        f,
        p,
    )
    .unwrap();
}
#[test]
fn original_status_enum_resolves_full_parent_packets_without_changing_passive_mount() {
    let cases = [
        (None, "content"),
        (Some(StatusBadge::Info), "content-status-info"),
        (Some(StatusBadge::Running), "content-status-running"),
        (Some(StatusBadge::Succeeded), "content-status-succeeded"),
        (Some(StatusBadge::Failed), "content-status-failed"),
    ];
    for mode in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                let p = Palette::new(mode, brand).unwrap();
                let mut h = ContentHost::new(Content::new(
                    ComponentId(1),
                    IconContent::new("tekes-back")
                        .unwrap()
                        .with_accessible_name("State"),
                ))
                .unwrap();
                h.set_inherited_surface(surface).unwrap();
                let mount = h.semantic_mount(ComponentId(1));
                let mut icons = Icons::default();
                for (tone, usage) in cases {
                    for disabled in [false, true] {
                        let icon = h
                            .get_mut(ComponentId(1))
                            .unwrap()
                            .component_mut::<IconContent>()
                            .unwrap();
                        icon.set_status_tone(tone);
                        icon.set_disabled(disabled);
                        assert_eq!(icon.status_tone(), tone);
                        assert_eq!(icon.is_disabled(), disabled);
                        paint(&mut h, &mut icons, p);
                        let packet = resolve_surface(
                            p,
                            surface,
                            usage,
                            if disabled { "disabled" } else { "rest" },
                        )
                        .unwrap();
                        assert_eq!(icons.ink, [packet.icon]);
                        assert_eq!(
                            packet.background,
                            resolve_surface(p, surface, "content", "rest")
                                .unwrap()
                                .background
                        );
                        assert!(
                            packet.focus.is_none()
                                && packet.boundary.is_none()
                                && packet.marker.is_none()
                                && packet.helper.is_none()
                        );
                        let ax = h.semantics();
                        assert_eq!(ax.len(), 1);
                        assert_eq!(ax[0].semantics.name, "State");
                        assert_eq!(ax[0].semantics.disabled, disabled);
                        assert!(h.focus_targets().is_empty());
                        assert_eq!(h.semantic_mount(ComponentId(1)), mount);
                    }
                }
                let icon = h
                    .get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap();
                assert!(icon.set_accessible_name(Some(" ".into())).is_err());
                assert!(icon.set_accessible_name(Some("Failed".into())).unwrap());
                assert_eq!(h.semantics()[0].semantics.name, "Failed");
                h.get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap()
                    .set_accessible_name(None)
                    .unwrap();
                assert!(h.semantics().is_empty());
                assert_eq!(h.semantic_mount(ComponentId(1)), mount);
            }
        }
    }
}
#[test]
fn decorative_status_never_overrides_original_action_ink_in_any_original_phase() {
    for mode in Theme::ALL {
        for brand in [0, 1, 2] {
            for (variant, usage) in [
                (ButtonVariant::Primary, "action-primary"),
                (ButtonVariant::Quiet, "action-quiet"),
                (ButtonVariant::Panel, "action-quiet"),
            ] {
                for tone in [
                    StatusBadge::Info,
                    StatusBadge::Running,
                    StatusBadge::Succeeded,
                    StatusBadge::Failed,
                ] {
                    for phase in [
                        "rest",
                        "hover",
                        "pressed",
                        "disabled",
                        "selected",
                        "selected-hover",
                        "selected-pressed",
                        "selected-disabled",
                        "loading",
                    ] {
                        for declared_surface in [false, true] {
                            let p = Palette::new(mode, brand).unwrap();
                            let mut icon = IconContent::new("tekes-settings")
                                .unwrap()
                                .with_accessible_name("inner status");
                            icon.set_status_tone(Some(tone));
                            icon.set_disabled(true);
                            let mut b = Button::new(Content::new(ComponentId(2), icon))
                                .with_accessible_name("Execute");
                            b.variant = variant;
                            b.button.state.selected = phase.starts_with("selected");
                            let mut h = ContentHost::new(Content::new(ComponentId(1), b)).unwrap();
                            h.get_mut(ComponentId(1))
                                .unwrap()
                                .set_surface(declared_surface.then_some("canvas"))
                                .unwrap();
                            let mut icons = Icons::default();
                            paint(&mut h, &mut icons, p);
                            let point = h.get(ComponentId(1)).unwrap().frame().center();
                            if phase.contains("hover") {
                                h.event(&ContentEvent::PointerMove(point));
                            }
                            if phase.contains("pressed") {
                                h.event(&ContentEvent::PointerDown(point));
                            }
                            let b = h
                                .get_mut(ComponentId(1))
                                .unwrap()
                                .component_mut::<Button>()
                                .unwrap();
                            b.button.state.disabled = phase.contains("disabled");
                            b.button.state.loading = phase == "loading";
                            paint(&mut h, &mut icons, p);
                            let expected = resolve_surface(
                                p,
                                "canvas",
                                usage,
                                if phase == "loading" { "rest" } else { phase },
                            )
                            .unwrap();
                            // Main's undeclared-Surface compatibility path shares the
                            // original label foreground. Explicit Surface uses its
                            // generated packet. Status decoration must obey both.
                            let expected_icon = if declared_surface {
                                expected.icon
                            } else {
                                p.color(if phase.contains("disabled") {
                                    "tertiary-text"
                                } else if variant == ButtonVariant::Primary {
                                    "action-primary-foreground"
                                } else {
                                    "primary-text"
                                })
                                .unwrap()
                            };
                            assert_eq!(
                                icons.ink,
                                [expected_icon],
                                "mode={mode:?} brand={brand} variant={variant:?} tone={tone:?} phase={phase} declared_surface={declared_surface}"
                            );
                            let ax = h.semantics();
                            assert_eq!(ax.len(), 1);
                            assert_eq!(ax[0].semantics.name, "Execute");
                        }
                    }
                }
            }
        }
    }
}
// This is run explicitly in a separately frozen old-Theme derived source.
#[test]
#[ignore = "old Theme1.5.3 source capability rejection; not a positive candidate test"]
fn missing_status_packet_is_explicit_without_paint_fallback() {
    let mut icon = IconContent::new("tekes-back").unwrap();
    icon.set_status_tone(Some(StatusBadge::Failed));
    let mut h = ContentHost::new(Content::new(ComponentId(1), icon)).unwrap();
    let f = Rect::new(0., 0., 32., 32.);
    let mut s = Scene::new();
    let mut icons = Icons::default();
    assert_eq!(
        h.render(
            &mut s,
            &mut WithIconRenderer::new(&mut Text, &mut icons),
            f,
            f,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::IconResource {
            owner: ComponentId(1),
            reason: IconRenderError::UnsupportedTone
        })
    );
    assert!(icons.ink.is_empty());
    assert!(!h.animation_frame_requested());
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_status_tone(None);
    paint(&mut h, &mut icons, Theme::LIGHT.into());
    assert_eq!(icons.ink.len(), 1);
}
