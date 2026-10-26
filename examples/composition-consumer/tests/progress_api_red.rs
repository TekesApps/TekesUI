use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, ProgressContent};
#[test]
fn public_progress_is_mountable() {
    let _ = ContentHost::new(Content::new(
        ComponentId(58002),
        ProgressContent::new("Progress", 0.45),
    ))
    .unwrap();
}
