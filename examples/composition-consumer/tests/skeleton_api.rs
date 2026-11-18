use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, SkeletonContent};
#[test]
fn public_skeleton() {
    ContentHost::new(Content::new(ComponentId(59001), SkeletonContent::default())).unwrap();
}
