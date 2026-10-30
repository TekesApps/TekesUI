//! Bridge witness using original finite models. This is not any of the #108–110 adapters.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextRenderer, CheckState, Checkbox,
    composition::*,
    extended::{RadioGroup, SegmentedControl},
};
use tekes_ui_platform_macos::{AccessibilityRole, content_accessibility as ax};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::Theme;
#[derive(Debug)]
struct Witness {
    role: ContentRole,
    check: Checkbox,
    radio: RadioGroup,
    segment: SegmentedControl,
    calls: usize,
    disabled: bool,
    bad_parent: bool,
    frame: Rect,
}
impl Witness {
    fn new(role: ContentRole) -> Self {
        Self {
            role,
            check: Checkbox {
                checked: CheckState::Mixed,
                ..Default::default()
            },
            radio: RadioGroup::new(2),
            segment: SegmentedControl::new(2),
            calls: 0,
            disabled: false,
            bad_parent: false,
            frame: Rect::default(),
        }
    }
}
impl Component for Witness {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 40.))
    }
    fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
        self.frame = frame;
    }
    fn focusable(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(self.role, "Witness");
        s.disabled = self.disabled;
        if self.role == ContentRole::Checkbox {
            s.selected = self.check.checked == CheckState::Checked;
            s.value = Some(
                match self.check.checked {
                    CheckState::Mixed => "mixed",
                    CheckState::Checked => "on",
                    CheckState::Unchecked => "off",
                }
                .into(),
            );
        }
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if self.role == ContentRole::Checkbox {
            return vec![];
        }
        vec![SemanticPart {
            index: 1,
            frame: self.frame,
            focused: false,
            semantics: {
                let mut s = ContentSemantics::new(
                    if self.bad_parent {
                        ContentRole::Segment
                    } else if self.role == ContentRole::RadioGroup {
                        ContentRole::RadioButton
                    } else {
                        ContentRole::Segment
                    },
                    "Choice",
                );
                s.selected = if self.role == ContentRole::RadioGroup {
                    self.radio.selected == Some(1)
                } else {
                    self.segment.selected == 1
                };
                s
            },
        }]
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        self.calls += 1;
        let changed = if matches!(a, AccessibilityAction::Press) {
            if part.is_none() {
                self.check.toggle()
            } else if self.role == ContentRole::RadioGroup {
                self.radio.select(1)
            } else {
                self.segment.select(1)
            }
        } else {
            false
        };
        ContentResponse {
            consumed: true,
            changed,
            activated: changed,
            ..Default::default()
        }
    }
}
fn setup(role: ContentRole) -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        ListContent::new(
            "Root",
            vec![
                Content::new(ComponentId(2), Witness::new(role)),
                Content::new(ComponentId(3), TextFieldContent::new("Composer", "draft")),
            ],
        ),
    ))
    .unwrap();
    paint(&mut h);
    h
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 320., 180.);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    h.render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn witness(h: &ContentHost) -> &Witness {
    h.get(ComponentId(2))
        .unwrap()
        .component::<Witness>()
        .unwrap()
}
#[test]
fn public_roles_project_original_values_and_only_finite_owner_actions() {
    for role in [
        ContentRole::Checkbox,
        ContentRole::RadioGroup,
        ContentRole::SegmentedGroup,
    ] {
        let mut h = setup(role);
        let key = ax::content_key(&h, ComponentId(2), None).unwrap();
        let nodes = ax::content_nodes(&h);
        let n = nodes.iter().find(|n| n.key == key).unwrap();
        assert_eq!(
            n.role,
            if role == ContentRole::Checkbox {
                AccessibilityRole::Checkbox
            } else {
                AccessibilityRole::RadioGroup
            }
        );
        assert!(h.semantic_text_input_snapshot(ComponentId(2)).is_none());
        assert!(
            h.semantic_text_layout_snapshot(ComponentId(2), None)
                .is_none()
        );
        assert!(h.semantic_value_range(ComponentId(2), None).is_none());
        let part = (role != ContentRole::Checkbox).then_some(1);
        let key = ax::content_key(&h, ComponentId(2), part).unwrap();
        if part.is_some() {
            assert_eq!(
                nodes.iter().find(|n| n.key == key).unwrap().role,
                AccessibilityRole::RadioButton
            );
            assert!(
                !h.accessibility_action(ComponentId(2), None, AccessibilityAction::Press)
                    .consumed
            );
        }
        for action in [
            AccessibilityAction::SetValue("on".into()),
            AccessibilityAction::SetSelection(0..1),
            AccessibilityAction::ReplaceSelectedText("x".into()),
            AccessibilityAction::Increment,
            AccessibilityAction::ShowMenu,
            AccessibilityAction::SetExpanded(true),
        ] {
            assert!(
                !h.accessibility_action(ComponentId(2), part, action)
                    .consumed
            );
        }
        assert_eq!(witness(&h).calls, 0);
        assert!(
            ax::content_action(
                &mut h,
                key,
                tekes_ui_platform_macos::AccessibilityAction::Press
            )
            .activated
        );
        assert_eq!(witness(&h).calls, 1);
        if role == ContentRole::Checkbox {
            assert_eq!(witness(&h).check.checked, CheckState::Checked);
        } else if role == ContentRole::RadioGroup {
            assert_eq!(witness(&h).radio.selected, Some(1));
        } else {
            assert_eq!(witness(&h).segment.selected, 1);
        }
    }
}
#[test]
fn original_host_ime_scope_disabled_and_parent_gates_precede_owner_mutation() {
    for role in [
        ContentRole::Checkbox,
        ContentRole::RadioGroup,
        ContentRole::SegmentedGroup,
    ] {
        let mut h = setup(role);
        assert!(h.focus(Some(ComponentId(3)), false));
        h.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        });
        let before = h.text_input_snapshot().unwrap();
        assert!(before.marked.is_some());
        let epoch = h.focus_epoch();
        let binding = h.native_key_binding();
        let part = (role != ContentRole::Checkbox).then_some(1);
        for a in [AccessibilityAction::Focus, AccessibilityAction::Press] {
            assert!(!h.accessibility_action(ComponentId(2), part, a).consumed);
        }
        assert_eq!(witness(&h).calls, 0);
        assert_eq!(h.text_input_snapshot().unwrap(), before);
        assert_eq!(h.focus_epoch(), epoch);
        assert_eq!(h.native_key_binding(), binding);
        h.set_window_active(false);
        assert!(
            !h.accessibility_action(ComponentId(2), part, AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(witness(&h).calls, 0);
    }
    let mut h = setup(ContentRole::RadioGroup);
    h.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<Witness>()
        .unwrap()
        .bad_parent = true;
    assert!(
        !h.accessibility_action(ComponentId(2), Some(1), AccessibilityAction::Focus)
            .consumed
    );
    assert!(
        !h.accessibility_action(ComponentId(2), Some(1), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(witness(&h).calls, 0);
    assert_eq!(h.focused(), None);
    h.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<Witness>()
        .unwrap()
        .disabled = true;
    assert!(
        !h.accessibility_action(ComponentId(2), Some(1), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(witness(&h).calls, 0);
}
