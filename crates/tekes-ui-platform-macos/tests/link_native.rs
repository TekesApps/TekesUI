#[cfg(target_os = "macos")]
fn main() {
    let checks = tekes_ui_platform_macos::selftest::link_role_contract();
    assert_eq!(checks, 6);
    let image_checks = tekes_ui_platform_macos::selftest::image_role_contract();
    assert_eq!(image_checks, 3);
    let splitter_checks = tekes_ui_platform_macos::selftest::splitter_role_contract();
    assert_eq!(splitter_checks, 18);
    println!(
        "SPLITTER_NATIVE_OBJECT_CONTRACT_OK checks={splitter_checks} hidden_windows=1 shown_windows=0 desktop_actions=0"
    );
    println!("IMAGE_NATIVE_OBJECT_CONTRACT_OK checks={image_checks} windows=0 desktop_actions=0");
    println!("LINK_NATIVE_OBJECT_CONTRACT_OK checks={checks} windows=0 desktop_actions=0");
    #[cfg(feature = "content-accessibility")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, TextFieldContent};
        use tekes_ui_controls::{
            ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
        };
        use tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot;
        use tekes_ui_scene::{Point, Rect, Scene, Size};
        use tekes_ui_text::{
            Paragraph, ShapeOptions, Shaper, TextStyle,
            coretext::{CoreTextFonts, CoreTextShaper},
        };
        use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

        struct DummyText(CoreTextShaper);
        impl ButtonTextRenderer for DummyText {
            fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
                Some(self)
            }
            fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
                let paragraph = self.shape_surface(text, role, Some(width.max(1.0)));
                Size::new(paragraph.width as f32, paragraph.height as f32)
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
        impl TextSurfaceHost for DummyText {
            fn shape_surface(
                &mut self,
                text: &str,
                role: proposal::TextRole,
                width: Option<f32>,
            ) -> Rc<Paragraph> {
                Rc::new(self.0.shape(
                    text,
                    &TextStyle::system(role.style().size).with_weight(role.style().weight),
                    &ShapeOptions {
                        width: width.map(f64::from),
                        line_spacing: 0.,
                    },
                ))
            }
        }
        impl TextSurfaceTextRenderer for DummyText {
            fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
        }

        let host = ContentHost::new(Content::new(
            ComponentId(1),
            TextFieldContent::new("Search", "a"),
        ))
        .unwrap();
        let text = DummyText(CoreTextShaper::new(CoreTextFonts::new()));
        let frame = Rect::new(0.0, 0.0, 200.0, 100.0);
        let snap = move |host: &mut ContentHost, text: &mut DummyText| {
            let mut scene = Scene::new();
            host.render(&mut scene, text, frame, frame, Palette::from(Theme::LIGHT))
                .unwrap();
            ContentAccessibilitySnapshot::from_host(host).unwrap()
        };
        let harness = Rc::new(RefCell::new((host, text)));
        let first = {
            let mut guard = harness.borrow_mut();
            let (host, text) = &mut *guard;
            snap(host, text)
        };
        let harness2 = harness.clone();
        let pub_checks =
            tekes_ui_platform_macos::selftest::content_accessibility_publication_contract(
                first,
                move || {
                    let mut guard = harness2.borrow_mut();
                    let (host, text) = &mut *guard;
                    host.focus(Some(ComponentId(1)), false);
                    host.event_with_text(
                        &tekes_ui_controls::composition::ContentEvent::Commit {
                            text: "b".into(),
                            replacement: None,
                        },
                        text,
                    )
                    .unwrap();
                    snap(host, text)
                },
            );
        assert_eq!(pub_checks, 4);
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("LINK_NATIVE_OBJECT_UNAVAILABLE target_is_not_macos");
}
