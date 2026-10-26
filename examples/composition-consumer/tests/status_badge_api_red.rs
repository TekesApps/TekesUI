use tekes_ui_controls::{
    composition::StatusBadgeContent,
    extended::{StatusBadge, status_badge_on_surface},
    feedback_view::StatusBadgeView,
};
#[test]
fn public_badge_api() {
    let _ = StatusBadgeContent::new(StatusBadge::Info, "Info").unwrap();
    let _ = StatusBadgeView::default().surface();
    status_badge_on_surface(
        &mut tekes_ui_scene::Scene::new(),
        tekes_ui_scene::Rect::new(0., 0., 80., 28.),
        tekes_ui_tokens::Theme::LIGHT,
        StatusBadge::Info,
        "canvas",
    )
    .unwrap();
}
