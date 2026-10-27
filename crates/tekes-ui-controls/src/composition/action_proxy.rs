//! Minimal authorization state for a fixed proxy over an original action owner.
//! Mount allocation and queue filtering remain in the original Content engine.
use super::*;
#[derive(Debug, Default)]
pub(super) struct Exposure {
    pending: bool,
    reserved: bool,
}
impl Exposure {
    pub(super) fn revoke(&mut self) {
        self.pending = true;
        self.reserved = false;
    }
    pub(super) fn replaced(&mut self) {
        self.pending = true;
        self.reserved = true;
    }
    pub(super) fn pending(&self) -> bool {
        self.pending
    }
    pub(super) fn published(&mut self) {
        self.pending = false;
    }
    pub(super) fn reserve(&self) -> Result<Option<u64>, CompositionError> {
        (self.pending && !self.reserved)
            .then(actions::try_next_mount)
            .transpose()
    }
}
