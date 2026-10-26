use tekes_ui_controls::composition::{SkeletonContent,ContentHost,Content,ComponentId};
#[test]fn public_skeleton(){ContentHost::new(Content::new(ComponentId(59001),SkeletonContent::default())).unwrap();}
