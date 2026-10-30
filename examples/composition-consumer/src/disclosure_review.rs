//! Public Disclosure acceptance scene. Only business confirmation policy lives here.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(38000);
pub const DISCLOSURE: ComponentId = ComponentId(38001);
pub const TRIGGER: ComponentId = ComponentId(38002);
pub const TITLE: ComponentId = ComponentId(38003);
pub const BODY: ComponentId = ComponentId(38004);
pub const EDITOR: ComponentId = ComponentId(38005);
pub const BODY_BUTTON: ComponentId = ComponentId(38006);
pub const OUTSIDE: ComponentId = ComponentId(38007);
pub const TITLE_DETAIL: ComponentId = ComponentId(38008);
pub const NAME: &str = "长期项目分组：摘要、正在进行的任务与中文说明";
pub const DRAFT: &str = "Original body draft";
fn stack(children: Vec<Content>) -> StackContent {
    StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 8.,
        },
        flexible_child: None,
        children,
    }
}
pub fn build() -> Result<ContentHost, CompositionError> {
    let title = Content::new(
        TITLE,
        stack(vec![
            Content::new(ComponentId(38009), TextContent::new(NAME)),
            Content::new(
                TITLE_DETAIL,
                TextContent::new("被动标题详情 · multilingual résumé"),
            ),
        ]),
    );
    let body = Content::new(
        BODY,
        stack(vec![
            Content::new(
                EDITOR,
                TextFieldContent::new("Disclosure body editor", DRAFT),
            ),
            Content::new(BODY_BUTTON, Button::new("Original body action")),
        ]),
    );
    let mut disclosure = DisclosureContent::new(TRIGGER, title, body, false);
    disclosure.set_accessible_name(NAME)?;
    let mut host = ContentHost::new(Content::new(
        ROOT,
        stack(vec![
            Content::new(DISCLOSURE, disclosure),
            Content::new(
                OUTSIDE,
                TextFieldContent::new("Unrelated editor", "Keep A😀B"),
            ),
        ]),
    ))?;
    host.set_action_events(true);
    Ok(host)
}
pub fn disclosure(host: &ContentHost) -> &DisclosureContent {
    host.get(DISCLOSURE).unwrap().component().unwrap()
}
pub fn disclosure_mut(host: &mut ContentHost) -> &mut DisclosureContent {
    host.get_mut(DISCLOSURE).unwrap().component_mut().unwrap()
}
pub fn intent(record: &ContentActionRecord) -> &DisclosureIntent {
    match record.payload.as_ref().expect("Disclosure action payload") {
        ContentActionPayload::DisclosureExpansion(intent) => intent,
        _ => panic!("expected original Disclosure intent"),
    }
}
