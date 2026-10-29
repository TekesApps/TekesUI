//! Reconstructible business document: each window owns different ComponentIds.
//! Persistence and business mapping belong to this consumer; geometry does not.
use tekes_ui_controls::{
    ReadingDocumentVersion, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
};

pub struct Reader {
    pub host: ContentHost,
    pub owner: ComponentId,
    pub list: ComponentId,
    pub editor: ComponentId,
    pub keys: Vec<(String, ComponentId)>,
    pub version: ReadingDocumentVersion,
}
impl Reader {
    pub fn new(base: u64, revision: u64, deleted: &[usize], unloaded: &[usize]) -> Self {
        let owner = ComponentId(base + 1);
        let list = ComponentId(base + 2);
        let editor = ComponentId(base + 3);
        let keys: Vec<_> = (0..24)
            .filter(|n| !deleted.contains(n))
            .map(|n| {
                (
                    format!("entry-{n}"),
                    ComponentId(base + 100 + n as u64 * 10),
                )
            })
            .collect();
        let children = keys
            .iter()
            .filter(|(key, _)| {
                let n = key
                    .strip_prefix("entry-")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                !unloaded.contains(&n)
            })
            .map(|(key, id)| {
                Content::new(
                    *id,
                    StackContent {
                        model: Stack {
                            axis: Axis::Vertical,
                            gap: 4.0,
                        },
                        flexible_child: None,
                        children: vec![
                            Content::new(
                                ComponentId(id.0 + 1),
                                TextContent::new(format!(
                                    "{key} 中文 😀 local reading checkpoint. {}",
                                    "A paragraph wraps with width and font changes. ".repeat(5)
                                )),
                            ),
                            Content::new(
                                ComponentId(id.0 + 2),
                                Button::new(format!("Inspect {key}")),
                            ),
                            Content::new(
                                ComponentId(id.0 + 3),
                                SwitchContent::new("Details", false),
                            ),
                        ],
                    },
                )
            })
            .collect();
        let mut host = ContentHost::new(Content::new(
            ComponentId(base),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.0,
                },
                flexible_child: Some(owner),
                children: vec![
                    Content::new(
                        owner,
                        ScrollContent::new(
                            ScrollArea::default(),
                            Content::new(list, ListContent::new("Saved history", children)),
                        ),
                    ),
                    Content::new(editor, TextFieldContent::new("Draft", "Untouched 中文")),
                ],
            },
        ))
        .unwrap();
        let (version, ()) = host
            .set_scroll_reading_document(
                owner,
                "business-document",
                revision,
                keys.iter().map(|(_, id)| *id).collect(),
                |_| (),
            )
            .unwrap();
        Self {
            host,
            owner,
            list,
            editor,
            keys,
            version,
        }
    }
    pub fn scroll(&self) -> &ScrollContent {
        self.host
            .get(self.owner)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
    }
    pub fn capture(&self) -> Result<ContentReadingCheckpoint, ContentReadingCheckpointError> {
        self.host
            .capture_scroll_reading_checkpoint(self.owner, &self.version, &self.keys)
    }
    pub fn request(&self) -> ContentReadingRestoreRequest {
        self.host
            .prepare_scroll_reading_restore(self.owner, &self.version)
            .unwrap()
    }
    pub fn restore(
        &mut self,
        request: &ContentReadingRestoreRequest,
        value: &ContentReadingCheckpoint,
    ) -> Result<(), ContentReadingCheckpointError> {
        self.host
            .restore_scroll_reading_checkpoint(request, value, &self.keys)
    }
    pub fn id(&self, key: &str) -> ComponentId {
        self.keys.iter().find(|(k, _)| k == key).unwrap().1
    }
}
