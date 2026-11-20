//! Color role label formatter shared with AppHost Foundations.

use tekes_ui_tokens::ColorRole;

/// 色块名：`ScrimX` 这样的驼峰拆成 `Scrim X`，让换行落在词边界上。
pub fn role_label(role: ColorRole) -> String {
    let name = format!("{role:?}");
    let mut out = String::with_capacity(name.len() + 3);
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{
        text::TextEngine,
        workbench::{self, apphost_foundations::AppHostFoundations},
    };
    use tekes_ui_controls::composition::{AccessibilityAction, ComponentId, ContentHost};
    use tekes_ui_scene::{Primitive, Rect, Scene};
    use tekes_ui_tokens::Theme;

    fn collect_quads(scene: &Scene) -> Vec<tekes_ui_scene::Quad> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Quad(q) => Some(*q),
                _ => None,
            })
            .collect()
    }

    fn press_token_atlas(host: &mut ContentHost, text: &mut TextEngine, frame: Rect) {
        let mut scene = Scene::new();
        host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        assert!(
            host.accessibility_action(ComponentId(97811), None, AccessibilityAction::Press)
                .consumed
        );
    }

    #[test]
    fn apphost_swatch_grid_holds_all_27_roles_in_enum_order_and_fits_viewport() {
        assert_eq!(ColorRole::ALL.len(), 27);
        let entry = workbench::entries()
            .iter()
            .find(|e| e.renderer() == "tokens")
            .unwrap();
        let mut page = AppHostFoundations::new(entry).unwrap();
        let mut text = TextEngine::new();
        let frame = Rect::new(0.0, 0.0, 820.0, 960.0);
        press_token_atlas(&mut page.host, &mut text, frame);
        page.after_host_visit();

        for theme in Theme::ALL {
            let mut scene = Scene::new();
            page.host
                .render(&mut scene, &mut text, frame, frame, theme.into())
                .unwrap();
            let all_quads = collect_quads(&scene);
            for (i, role) in ColorRole::ALL.iter().enumerate() {
                let sample = page.host.get(ComponentId(98001 + i as u64)).unwrap();
                let rect = sample.frame();
                assert!(rect.min_x() >= frame.min_x() && rect.max_x() <= frame.max_x());
                let swatch = all_quads
                    .iter()
                    .filter(|q| q.rect.intersection(&rect).is_some())
                    .find(|q| q.fill == role.resolve(theme));
                assert!(
                    swatch.is_some(),
                    "AppHost swatch {i} must render {role:?} in {theme:?}"
                );
            }
        }
    }

    #[test]
    fn color_role_labels_split_camel_case() {
        assert_eq!(
            role_label(ColorRole::FloatingPanelDivider),
            "Floating Panel Divider"
        );
        assert_eq!(role_label(ColorRole::Scrim), "Scrim");
        for role in ColorRole::ALL {
            assert!(!role_label(role).is_empty());
        }
    }
}
