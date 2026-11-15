//! Opt-in, thread-local observation of actual public lookup calls.
//! Records requested semantic IDs. Optional resolved-color observation records
//! the actual Palette lookup result, not alias closure or proof of pixel use.
use std::{cell::RefCell, collections::BTreeSet};

type Reads = BTreeSet<(String, String)>;
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedColorRead {
    pub role: String,
    pub palette: crate::Palette,
    pub value: crate::Rgba,
}
#[derive(Clone, Debug, Default)]
pub struct TokenObservation {
    pub lookups: Vec<(String, String)>,
    pub colors: Vec<ResolvedColorRead>,
}
struct ObservationScope {
    reads: Reads,
    colors: Option<Vec<ResolvedColorRead>>,
}
thread_local! {
    static SCOPES: RefCell<Vec<ObservationScope>> = const { RefCell::new(Vec::new()) };
}
struct Scope(bool);
impl Drop for Scope {
    fn drop(&mut self) {
        if self.0 {
            SCOPES.with(|scopes| {
                scopes.borrow_mut().pop();
            });
        }
    }
}
pub(crate) fn record(family: &str, id: &str) {
    SCOPES.with(|scopes| {
        if let Some(reads) = scopes.borrow_mut().last_mut() {
            reads.reads.insert((family.to_owned(), id.to_owned()));
        }
    });
}
/// Isolate a synchronous render/measure call; nested scopes and panics do not
/// contaminate a later component's result. No recording occurs outside a scope.
pub fn capture<T>(render: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    let (value, observation) = capture_inner(render, false);
    (value, observation.lookups)
}
/// Observe actual resolved Palette colors during this call. Nested observations
/// remain isolated. No value is recomputed after the observed operation.
pub fn capture_resolved<T>(render: impl FnOnce() -> T) -> (T, TokenObservation) {
    capture_inner(render, true)
}
pub(crate) fn record_color(role: &str, palette: crate::Palette, value: crate::Rgba) {
    SCOPES.with(|scopes| {
        let mut scopes = scopes.borrow_mut();
        if let Some(colors) = scopes.last_mut().and_then(|scope| scope.colors.as_mut()) {
            let read = ResolvedColorRead {
                role: role.to_owned(),
                palette,
                value,
            };
            if !colors.contains(&read) {
                colors.push(read);
            }
        }
    });
}
fn capture_inner<T>(render: impl FnOnce() -> T, colors: bool) -> (T, TokenObservation) {
    SCOPES.with(|scopes| {
        scopes.borrow_mut().push(ObservationScope {
            reads: Reads::new(),
            colors: colors.then(Vec::new),
        })
    });
    let mut scope = Scope(true);
    let value = render();
    let reads = SCOPES.with(|scopes| scopes.borrow_mut().pop().unwrap());
    scope.0 = false;
    (
        value,
        TokenObservation {
            lookups: reads.reads.into_iter().collect(),
            colors: reads.colors.unwrap_or_default(),
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolved_colors_keep_actual_palette_and_nested_scope_identity() {
        let light = crate::Palette::from(crate::Theme::LIGHT);
        let dark = crate::Palette::from(crate::Theme::DARK);
        let (value, outer) = capture_resolved(|| {
            let value = light.color("border").unwrap();
            let (nested, inner) = capture_resolved(|| dark.color("border").unwrap());
            assert_eq!(
                inner.colors,
                [ResolvedColorRead {
                    role: "border".into(),
                    palette: dark,
                    value: nested
                }]
            );
            assert!(light.color("unknown-inspection-role").is_none());
            value
        });
        assert_eq!(
            outer.colors,
            [ResolvedColorRead {
                role: "border".into(),
                palette: light,
                value
            }]
        );
        assert!(capture_resolved(|| ()).1.colors.is_empty());
    }
    #[test]
    fn scopes_are_thread_local_nested_and_panic_safe() {
        let (_, outer) = capture(|| {
            crate::proposal::color("border", crate::Theme::LIGHT).unwrap();
            let (_, inner) = capture(|| crate::proposal::metric("stroke", "stroke-rule").unwrap());
            assert_eq!(inner, [("stroke".into(), "stroke-rule".into())]);
            let other =
                std::thread::spawn(|| capture(|| crate::proposal::text("ui-body").unwrap()).1)
                    .join()
                    .unwrap();
            assert_eq!(other, [("type".into(), "ui-body".into())]);
        });
        assert_eq!(outer, [("color".into(), "border".into())]);
        let failed = std::panic::catch_unwind(|| capture(|| panic!("fixture")));
        assert!(failed.is_err());
        assert!(capture(|| ()).1.is_empty());
    }
}
