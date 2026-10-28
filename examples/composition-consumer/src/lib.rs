//! Independent workspace, public APIs only. The embedding application supplies
//! its text renderer and registered approved-icon images. No Gallery imports.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
    layout_view::{IconRenderer, IconView},
    paint,
};
use tekes_ui_scene::{ImageId, ImageQuad, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Spacing, proposal};
#[derive(Debug)]
pub struct IconImages(pub Vec<(Rgba, ImageId)>);
impl IconRenderer for IconImages {
    fn draw_icon(&mut self, scene: &mut Scene, name: &str, frame: Rect, tint: Rgba) {
        assert_eq!(name, tekes_ui_icons::Icon::Settings.name());
        let image = self
            .0
            .iter()
            .find(|(ink, _)| *ink == tint)
            .expect("host must prepare approved icon for active foreground")
            .1;
        scene.push_image(ImageQuad {
            rect: frame,
            image,
            opacity: 1.,
        });
    }
}
#[derive(Debug)]
struct Icon {
    view: IconView,
    images: IconImages,
}
impl Component for Icon {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(24., 24.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view.render_tinted(
            scene,
            &mut self.images,
            tekes_ui_icons::Icon::Settings.name(),
            frame,
            24.,
            Some(clip),
            theme.palette,
            theme
                .foreground
                .unwrap_or_else(|| theme.palette.color("primary-text").unwrap()),
        );
    }
}
#[derive(Debug)]
struct Badge(String);
impl Component for Badge {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let s = text.measure(&self.0, proposal::TextRole::UiCaption, available.width);
        MeasuredChild::new(Size::new(
            s.width + Spacing::Regular.value(),
            s.height + Spacing::Tight.value(),
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        paint::surface_at_scale(scene, frame, theme.palette, true, text.backing_scale());
        text.draw(
            scene,
            &self.0,
            proposal::TextRole::UiCaption,
            frame,
            ButtonTextAlign::Center,
            theme.palette.color("primary-text").unwrap(),
        );
    }
}
pub fn compound_button(images: IconImages) -> Result<ContentHost, CompositionError> {
    let caption = Content::new(
        ComponentId(4),
        StackContent {
            flexible_child: None,
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Tight.value(),
            },
            children: vec![
                Content::new(
                    ComponentId(5),
                    TextContent {
                        text: "Workspace settings".into(),
                        role: proposal::TextRole::UiHeading,
                    },
                ),
                Content::new(
                    ComponentId(6),
                    TextContent {
                        text: "Theme, input and accessibility".into(),
                        role: proposal::TextRole::UiCaption,
                    },
                ),
            ],
        },
    );
    let row = Content::new(
        ComponentId(2),
        StackContent {
            flexible_child: Some(ComponentId(4)),
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            children: vec![
                Content::new(
                    ComponentId(3),
                    Icon {
                        view: IconView::default(),
                        images,
                    },
                ),
                caption,
                Content::new(ComponentId(7), Badge("Local".into())),
            ],
        },
    );
    ContentHost::new(Content::new(
        ComponentId(1),
        Button::new(row).with_accessible_name("Open workspace settings"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::ControlKey;
    use tekes_ui_tokens::{Palette, Theme};
    #[derive(Default)]
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
            Size::new((t.len() as f32 * 7.).min(w), r.style().line_height)
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
    #[test]
    fn consumer_owned_icon_badge_and_nested_text_are_one_library_action() {
        for theme in Theme::ALL {
            let palette = Palette::from(theme);
            let ink = palette.color("primary-text").unwrap();
            let mut host = compound_button(IconImages(vec![(ink, ImageId(1))])).unwrap();
            host.render(
                &mut Scene::new(),
                &mut Text,
                Rect::new(12., 12., 440., 120.),
                Rect::new(0., 0., 480., 160.),
                palette,
            )
            .unwrap();
            assert_eq!(host.focus_targets().len(), 1);
            assert_eq!(host.semantics().len(), 1);
            assert_eq!(
                host.semantics()[0].semantics.name,
                "Open workspace settings"
            );
            host.focus(Some(ComponentId(1)), true);
            assert!(
                host.event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .activated
            );
            assert_eq!(
                host.get(ComponentId(1))
                    .unwrap()
                    .component::<Button>()
                    .unwrap()

                    .activations,
                1
            );
        }
    }
}

pub mod scenarios;

pub mod content_status;

pub mod choice_status;

pub mod combo_status;

pub mod command_status;

pub mod reading_prepend;
pub mod slider_content;

pub mod controlled_editor;

pub mod text_viewport;

pub mod content_actions;

pub mod reading_stream;

pub mod field_metadata;
pub mod number_field;
pub mod plain_text;

pub mod controlled_reading;
pub mod popover_content;
pub mod scroll_updates;

pub mod feedback_status;

pub mod label_content;

pub mod layout_activation;

pub mod item_navigation;

pub mod compact_actions;

pub mod padding_focus;

pub mod forwarded_actions;

pub mod help_list;

pub mod declared_menu;

pub mod toolbar_semantics;

pub mod toolbar_groups;

pub mod split_toolbar;

pub mod editable_number;

pub mod progress_content;

pub mod status_badge_content;

pub mod skeleton_content;

pub mod empty_state_content;

pub mod toolbar_mount;

pub mod icon_content;

/// Original nested reader; the SDK owns routed history notifications.
pub mod nested_reading;
