//! Business declarations only; the SDK owns glyphs, input, help, AX and resources.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(20800);
pub const ICON: ComponentId = ComponentId(20801);
pub const BACK: ComponentId = ComponentId(20802);
pub const SWAP: ComponentId = ComponentId(20803);
pub const HIDE: ComponentId = ComponentId(20804);
pub const TONE: ComponentId = ComponentId(20810);
pub const UNAVAILABLE: ComponentId = ComponentId(20811);
pub const RUN: ComponentId = ComponentId(20809);
pub const COMPOSER: ComponentId = ComponentId(20805);
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut back = Button::new(Content::new(
        ComponentId(20806),
        IconContent::new("tekes-back").unwrap(),
    ))
    .with_accessible_name("Back");
    back.set_help(Some("Back · original Button".into()));
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            flexible_child: None,
            children: vec![
                Content::new(
                    ComponentId(20807),
                    TextContent::new("Canonical IconContent · one window resource service"),
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(TONE, Button::new("Cycle status")).preserving_focus(),
                Content::new(UNAVAILABLE, Button::new("Toggle unavailable")).preserving_focus(),
                Content::new(
                    ComponentId(20808),
                    StackContent {
                        model: Stack {
                            axis: Axis::Horizontal,
                            gap: 12.,
                        },
                        flexible_child: None,
                        children: vec![
                            Content::new(
                                ICON,
                                IconContent::new("tekes-settings")
                                    .unwrap()
                                    .with_accessible_name("Status icon"),
                            ),
                            Content::new(BACK, back).preserving_focus(),
                            Content::new(RUN, Button::new("Run / stop")).preserving_focus(),
                            Content::new(SWAP, Button::new("Swap glyph")).preserving_focus(),
                            Content::new(HIDE, Button::new("Hide icon")).preserving_focus(),
                        ],
                    },
                ),
            ],
        },
    ))?;
    host.set_action_events(true);
    Ok(host)
}
pub fn process_actions(host: &mut ContentHost) -> Vec<ContentActionRecord> {
    let records = host.drain_action_records();
    for record in &records {
        if record.action.kind != ContentActionKind::Activate {
            continue;
        }
        match record.action.target {
            SWAP => {
                let icon = host
                    .get_mut(ICON)
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap();
                let next = if icon.glyph().name == "tekes-settings" {
                    "tekes-back"
                } else {
                    "tekes-settings"
                };
                icon.set_glyph(next).unwrap();
            }
            TONE => {
                use tekes_ui_controls::extended::StatusBadge;
                let icon = host
                    .get_mut(ICON)
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap();
                let next = match icon.status_tone() {
                    None => Some(StatusBadge::Info),
                    Some(StatusBadge::Info) => Some(StatusBadge::Running),
                    Some(StatusBadge::Running) => Some(StatusBadge::Succeeded),
                    Some(StatusBadge::Succeeded) => Some(StatusBadge::Failed),
                    Some(StatusBadge::Failed) => None,
                };
                icon.set_status_tone(next);
            }
            UNAVAILABLE => {
                let icon = host
                    .get_mut(ICON)
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap();
                icon.set_disabled(!icon.is_disabled());
            }
            RUN => {
                let icon = host
                    .get_mut(ICON)
                    .unwrap()
                    .component_mut::<IconContent>()
                    .unwrap();
                icon.set_indeterminate(!icon.is_indeterminate());
            }
            HIDE => {
                let node = host.get_mut(ICON).unwrap();
                let icon = node.component_mut::<IconContent>().unwrap();
                icon.set_visible(!icon.active());
            }
            _ => {}
        }
    }
    records
}
