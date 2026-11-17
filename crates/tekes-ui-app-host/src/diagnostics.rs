//! Opt-in stderr diagnostics for `tekes-ui-app-host`, disabled by default in production.
use std::sync::OnceLock;

static DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();

pub(crate) fn enabled() -> bool {
    *DIAGNOSTICS_ENABLED.get_or_init(|| {
        std::env::var_os("TEKESUI_APPHOST_DIAGNOSTICS").is_some_and(|value| value == "1")
    })
}

macro_rules! host_diag {
    ($($arg:tt)*) => {{
        if $crate::diagnostics::enabled() {
            eprintln!($($arg)*);
        }
    }};
}
