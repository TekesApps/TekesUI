use super::super::text_demo::TextDemo;
use tekes_ui_tokens::Rgba;

const FAMILY_FIRST: usize = 700;
const CONFIRM: usize = 722;
const CANCEL: usize = 723;
const PREVIEW_ACTION: usize = 720;
pub(super) const ROLE_SAMPLES: [(&str, &str, &str); 3] = [
    (
        "Action",
        "action-primary-foreground",
        "action-primary-ground",
    ),
    ("Link", "c-link-foreground", "surface-0"),
    ("Subtle", "accent-subtle-foreground", "accent-subtle-ground"),
];

#[derive(Debug)]
pub(super) struct BrandPreview {
    direction: Option<String>,
    candidate: Option<usize>,
    input: TextDemo,
    demo_activations: usize,
}

impl Default for BrandPreview {
    fn default() -> Self {
        Self {
            direction: None,
            candidate: None,
            input: TextDemo::field("Type to preview candidate text", false, false),
            demo_activations: 0,
        }
    }
}

impl BrandPreview {
    pub(super) fn take_apphost_input(
        &mut self,
    ) -> tekes_ui_controls::composition::TextFieldContent {
        let input = std::mem::replace(&mut self.input, TextDemo::field("", false, false));
        input.into_content_fields().remove(0)
    }

    pub(super) fn candidate(&self) -> Option<usize> {
        self.candidate
    }
    pub(super) fn direction(&self) -> Option<&str> {
        self.direction.as_deref()
    }
    pub(super) fn demo_activations(&self) -> usize {
        self.demo_activations
    }
    pub(super) fn select_direction(&mut self, index: usize) {
        self.activate(FAMILY_FIRST + index);
    }
    pub(super) fn confirm_candidate(&mut self) -> Option<usize> {
        self.activate(CONFIRM).flatten()
    }
    pub(super) fn cancel_candidate(&mut self) {
        self.activate(CANCEL);
    }
    pub(super) fn activate_demo(&mut self) {
        self.activate(PREVIEW_ACTION);
    }

    fn activate(&mut self, id: usize) -> Option<Option<usize>> {
        self.input.blur();
        if (FAMILY_FIRST..FAMILY_FIRST + crate::gallery::brands::gallery_brand_directions().len())
            .contains(&id)
        {
            let direction = &crate::gallery::brands::gallery_brand_directions()[id - FAMILY_FIRST];
            self.direction = Some(direction.id.to_string());
            self.candidate = crate::gallery::brands::gallery_brand_presets()
                .iter()
                .find(|preset| preset.recommended_for_direction.contains(&direction.id))
                .map(|preset| preset.palette_index);
            return Some(None);
        }
        match id {
            CONFIRM => self.candidate.take().map(Some),
            CANCEL => {
                self.candidate = None;
                Some(None)
            }
            PREVIEW_ACTION => {
                self.demo_activations += 1;
                Some(None)
            }
            _ => None,
        }
    }
}

pub(super) fn parse_hex(hex: &str) -> Option<Rgba> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }
    let values = [1, 3, 5].map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok());
    Some(Rgba::rgb8(values[0]?, values[1]?, values[2]?))
}
