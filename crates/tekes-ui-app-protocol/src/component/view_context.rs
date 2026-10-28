//! Window environment values, not component state or a second design system.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewContext {
    pub type_scale: f32,
    pub dark: bool,
    pub increased_contrast: bool,
    /// Existing compiled Palette index; zero is the consumer BrandConfig.
    pub brand: usize,
    /// Semantic design-system identity, not wire colors. The SDK validates it
    /// against the compiled schema before changing the original View backend.
    #[serde(default = "default_surface")]
    pub surface: String,
    /// One host monotonic timeline. Never a child-owned animation clock.
    pub elapsed_nanos: u64,
    pub reduced_motion: bool,
    pub caret_visible: bool,
}
fn default_surface() -> String {
    "canvas".into()
}
impl Default for ViewContext {
    fn default() -> Self {
        Self {
            type_scale: 1.,
            dark: false,
            increased_contrast: false,
            brand: 0,
            surface: default_surface(),
            elapsed_nanos: 0,
            reduced_motion: false,
            caret_visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_is_typed_and_existing_service_envelope_stays_unchanged() {
        use crate::component::RequestBody;
        let value = ViewContext {
            type_scale: 1.5,
            elapsed_nanos: 123,
            dark: true,
            reduced_motion: true,
            surface: "toolbar".into(),
            ..Default::default()
        };
        let body = RequestBody::SetViewContext {
            view: 7,
            context: value.clone(),
        };
        let wire = serde_json::to_value(body).unwrap();
        assert_eq!(wire["kind"], "SetViewContext");
        let RequestBody::SetViewContext { view, context } = serde_json::from_value(wire).unwrap()
        else {
            panic!()
        };
        assert_eq!(view, 7);
        assert_eq!(context, value);
        assert_eq!(
            serde_json::to_value(RequestBody::Stop).unwrap(),
            serde_json::json!({"kind":"Stop"})
        );
        let mut legacy = serde_json::to_value(&value).unwrap();
        legacy.as_object_mut().unwrap().remove("surface");
        assert_eq!(
            serde_json::from_value::<ViewContext>(legacy)
                .unwrap()
                .surface,
            "canvas"
        );
        let mut extra = serde_json::to_value(value).unwrap();
        extra["private_editor"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ViewContext>(extra).is_err());
    }
}
