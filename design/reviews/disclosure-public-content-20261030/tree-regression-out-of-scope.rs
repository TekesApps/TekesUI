#[test]
fn canonical_tree_defaults_use_real_svg_cache_without_aliases() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    let mut text = text();
    let mut tree = tekes_ui_controls::extended::Tree::default();
    let root = tree.add("Root", None).unwrap();
    let branch = tree.add("Branch", Some(root)).unwrap();
    tree.add("Leaf", Some(branch)).unwrap();
    tree.toggle(root);
    let mut view = tekes_ui_controls::collections_view::TreeView::new();
    let mut scene = Scene::new();
    let frame = Rect::new(0., 0., 300., 150.);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        frame,
        Some(frame),
        Theme::LIGHT.into(),
    );
    let mut icons = cache.renderer(&mut gpu, 1.);
    view.render_icons(
        &mut scene,
        &mut icons,
        Some(frame),
        Theme::LIGHT.into(),
        16.,
    );
    assert_eq!(
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Image(_)))
            .count(),
        2
    );
}

