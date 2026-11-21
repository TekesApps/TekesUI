use tekes_ui_controls::{
    ButtonSize, ButtonTextRenderer, extended as controls, layout_view::BreadcrumbView,
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

const BODY: proposal::TextRole = proposal::TextRole::UiBody;

#[derive(Clone, Copy)]
struct BreadcrumbCase {
    title: &'static str,
    labels: &'static [&'static str],
    focused: Option<usize>,
    hovered: Option<usize>,
    focus_visible: bool,
    pressed: bool,
    disabled: bool,
}

fn breadcrumb_cases() -> [BreadcrumbCase; 6] {
    [
        BreadcrumbCase {
            title: "Default · current location is non-interactive",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · hover",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: Some(1),
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · pressed",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: Some(1),
            focus_visible: false,
            pressed: true,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · focus-visible",
            labels: &["Home", "Docs", "Drafts"],
            focused: Some(0),
            hovered: None,
            focus_visible: true,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Long mixed-language path",
            labels: &["工作区 Workspace", "客户资料 Client files"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Disabled · all ancestors",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: true,
        },
    ]
}

fn breadcrumb_height(text: &mut impl ButtonTextRenderer, labels: &[&str], width: f32) -> f32 {
    let mut remaining = width;
    let mut text_height = 0.0_f32;
    for (index, label) in labels.iter().enumerate() {
        let measured = tekes_ui_controls::ButtonModel::measure(
            text,
            label,
            remaining.max(0.),
            ButtonSize::Body,
            BODY,
        );
        text_height = text_height.max(measured.height);
        let cell_width = if index + 1 == labels.len() {
            remaining
        } else {
            (measured.width + 22.).min(remaining.max(0.))
        };
        remaining -= cell_width + if index + 1 < labels.len() { 8. } else { 0. };
    }
    text_height.max(34.)
}
fn paint_breadcrumb_case(
    text: &mut impl ButtonTextRenderer,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    case: BreadcrumbCase,
) {
    let BreadcrumbCase {
        labels,
        focused,
        hovered,
        focus_visible,
        pressed,
        disabled,
        ..
    } = case;
    let model = controls::Breadcrumb {
        items: labels.iter().map(|s| (*s).to_owned()).collect(),
    };
    let mut view = BreadcrumbView::default();
    view.set_window_active(true);
    view.set_disabled(disabled);
    let mut probe = Scene::new();
    let layout = view.render(
        &mut probe,
        text,
        &model,
        labels,
        frame,
        Some(clip),
        palette,
        BODY,
    );
    if let Some(index) = focused {
        view.focus_with_visibility(index, focus_visible);
    }
    if let Some(index) = hovered
        && let Some(target) = layout
            .items
            .get(index)
            .and_then(|item| item.visible_frame())
    {
        view.pointer_move(target.center());
        if pressed {
            view.pointer_down(target.center());
        }
    }
    view.render(
        scene,
        text,
        &model,
        labels,
        frame,
        Some(clip),
        palette,
        BODY,
    );
}
pub(in crate::gallery::workbench) fn apphost_breadcrumb_matrix()
-> Vec<tekes_ui_controls::composition::Content> {
    use tekes_ui_controls::composition::*;
    breadcrumb_cases()
        .into_iter()
        .enumerate()
        .map(|(i, case)| {
            StackContent::labeled_specimen(
                ComponentId(91750 + i as u64),
                ComponentId(91700 + i as u64 * 2),
                case.title,
                Content::new(ComponentId(91701 + i as u64 * 2), BreadcrumbFixture(case)),
            )
        })
        .collect()
}
struct BreadcrumbFixture(BreadcrumbCase);
impl std::fmt::Debug for BreadcrumbFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.title)
    }
}
impl tekes_ui_controls::composition::Component for BreadcrumbFixture {
    fn measure(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        let height = breadcrumb_height(
            &mut super::super::apphost_basic::FixtureText(text),
            self.0.labels,
            available.width,
        );
        tekes_ui_controls::layout::MeasuredChild::new(tekes_ui_scene::Size::new(
            available.width,
            height,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: tekes_ui_controls::composition::ContentTheme,
    ) {
        paint_breadcrumb_case(
            &mut super::super::apphost_basic::FixtureText(text),
            scene,
            frame,
            clip,
            theme.palette(),
            self.0,
        );
    }
}
