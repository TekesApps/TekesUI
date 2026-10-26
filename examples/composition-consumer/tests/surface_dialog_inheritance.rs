#[path = "support/coretext.rs"]
mod coretext;

use std::{cell::RefCell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        ButtonContent, Component, ComponentId, Content, ContentHost, ContentTheme, DialogContent,
        StackContent, TextContent,
    },
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Theme, surface_palette::canonical_surface};

type Surfaces = Rc<RefCell<Vec<&'static str>>>;

#[derive(Debug)]
struct SurfaceProbe(Surfaces);

impl Component for SurfaceProbe {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(90., 24.))
    }

    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.0.borrow_mut().push(theme.surface());
    }
}

fn render(host: &mut ContentHost, text: &mut coretext::Text) {
    let frame = Rect::new(0., 0., 400., 360.);
    host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
        .unwrap();
}

fn assert_dialog_surface(
    host: &mut ContentHost,
    text: &mut coretext::Text,
    seen: &Surfaces,
    expected: &str,
) {
    seen.borrow_mut().clear();
    render(host, text);
    let actual = seen.borrow();
    assert!(!actual.is_empty(), "dialog body probe must paint");
    assert!(
        actual.iter().all(|surface| *surface == expected),
        "dialog subtree should inherit {expected:?}, observed {actual:?}"
    );
}

#[test]
fn retained_dialog_inherits_ancestor_surface_and_respects_override_clear() {
    const ROOT: ComponentId = ComponentId(91001);
    const DIALOG: ComponentId = ComponentId(91002);
    let seen = Surfaces::default();
    let dialog = Content::new(
        DIALOG,
        DialogContent::new(
            "Surface dialog",
            Content::new(ComponentId(91003), TextContent::new("Header")),
            Content::new(ComponentId(91004), SurfaceProbe(seen.clone())),
            Content::new(ComponentId(91005), ButtonContent::new("Close")),
        ),
    );
    let root = Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            children: vec![dialog],
            flexible_child: None,
        },
    )
    .with_surface("sidebar")
    .unwrap();
    let mut host = ContentHost::new(root).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    render(&mut host, &mut text);
    host.show_dialog(DIALOG).unwrap();

    assert_dialog_surface(&mut host, &mut text, &seen, "sidebar");
    host.get_mut(ROOT)
        .unwrap()
        .set_surface(Some("code"))
        .unwrap();
    assert_dialog_surface(&mut host, &mut text, &seen, "code");

    host.get_mut(DIALOG)
        .unwrap()
        .set_surface(Some("hint"))
        .unwrap();
    assert_dialog_surface(&mut host, &mut text, &seen, "hint");
    host.get_mut(DIALOG).unwrap().set_surface(None).unwrap();
    assert_dialog_surface(&mut host, &mut text, &seen, "code");

    host.get_mut(ROOT).unwrap().set_surface(None).unwrap();
    host.set_inherited_surface("toolbar").unwrap();
    assert_dialog_surface(&mut host, &mut text, &seen, "toolbar");
}

#[test]
fn default_component_surface_is_resolved_and_unknown_names_fail_at_mount() {
    #[derive(Debug)]
    struct DeclaredSurface {
        name: &'static str,
        observed: Surfaces,
    }
    impl Component for DeclaredSurface {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(available)
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            theme: ContentTheme,
        ) {
            self.observed.borrow_mut().push(theme.surface());
        }
        fn default_surface(&self) -> Option<&'static str> {
            Some(self.name)
        }
    }

    let expected = canonical_surface("sidebar").unwrap();
    let observed = Surfaces::default();
    let root = Content::new(
        ComponentId(91010),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            children: vec![Content::new(
                ComponentId(91011),
                DeclaredSurface {
                    name: "sidebar",
                    observed: observed.clone(),
                },
            )],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(root).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    // A valid declaration is canonicalized by the same vocabulary used by explicit surfaces.
    render(&mut host, &mut text);
    assert_eq!(&*observed.borrow(), &[expected]);

    let invalid = Content::new(
        ComponentId(91012),
        DeclaredSurface {
            name: "Sidebar",
            observed,
        },
    );
    assert!(ContentHost::new(invalid).is_err());
}
