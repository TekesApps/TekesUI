// Original #38 deferred Tabs regression, enabled by #195.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::Theme;
const ROOT: ComponentId = ComponentId(195);
#[derive(Debug)]
struct DemandingClosedOverlay;
impl Component for DemandingClosedOverlay {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn active(&self) -> bool {
        false
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_presented(&self) -> bool {
        true
    }
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        Some(IconRenderError::Upload)
    }
}
#[test]
fn inactive_tabs_panel_does_not_revive_overlay_service_demand_or_failure() {
    let tabs = TabsContent::new(
        vec![
            TabItem::new(
                Content::new(ComponentId(11), TextContent::new("First")),
                Content::new(ComponentId(12), TextContent::new("Visible")),
            ),
            TabItem::new(
                Content::new(ComponentId(13), TextContent::new("Second")),
                Content::new(ComponentId(14), DemandingClosedOverlay),
            ),
        ],
        0,
    );
    let mut h = ContentHost::new(Content::new(ROOT, tabs)).unwrap();
    let mut t = coretext::Text::new(1.);
    let frame = Rect::new(0., 0., 250., 200.);
    assert_eq!(
        h.render(&mut Scene::new(), &mut t, frame, frame, Theme::LIGHT.into()),
        Ok(())
    );
}
