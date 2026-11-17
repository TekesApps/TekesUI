//! Original ProcessComponent registration identity, captured with a declaration.
use tekes_ui_module_runtime::{ProcessComponent, component::*};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentSource {
    pub session: Session,
    pub registration: u64,
    pub window: u64,
}
impl ComponentSource {
    pub fn current(process: &ProcessComponent) -> Option<Self> {
        let packet = process.toolbar_registration()?;
        if let Output::Event(ComponentEvent::Toolbar {
            revision,
            scope: ToolbarScope::Window { window },
            ..
        }) = packet.output
        {
            Some(Self {
                session: packet.session,
                registration: revision,
                window,
            })
        } else {
            None
        }
    }
    pub fn validate(
        &self,
        session: &Session,
        registration: u64,
        window: u64,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.session == *session && self.registration == registration && self.window == window,
            "stale or mismatched toolbar source"
        );
        Ok(())
    }
    pub fn invoke(&self, process: &ProcessComponent, command: u64) -> anyhow::Result<u64> {
        process.request_from(
            &self.session,
            RequestBody::Invoke {
                command,
                registration: self.registration,
                window: self.window,
            },
        )
    }
}
