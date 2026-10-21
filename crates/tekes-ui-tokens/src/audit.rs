//! Opt-in, thread-local observation of actual public lookup calls.
//! Records requested semantic IDs, not values, alias closure or timing. Constants
//! and motion/layout accessors are outside this diagnostic's scope.
use std::{cell::RefCell, collections::BTreeSet};

type Reads = BTreeSet<(String, String)>;
thread_local! {
    static SCOPES: RefCell<Vec<Reads>> = const { RefCell::new(Vec::new()) };
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
            reads.insert((family.to_owned(), id.to_owned()));
        }
    });
}
/// Isolate a synchronous render/measure call; nested scopes and panics do not
/// contaminate a later component's result. No recording occurs outside a scope.
pub fn capture<T>(render: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    SCOPES.with(|scopes| scopes.borrow_mut().push(Reads::new()));
    let mut scope = Scope(true);
    let value = render();
    let reads = SCOPES.with(|scopes| scopes.borrow_mut().pop().unwrap());
    scope.0 = false;
    (value, reads.into_iter().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
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
