//! Host execution receipts and local input gate; never owns entries/cursor/nav.json.
use super::*;
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet, VecDeque};
use tekes_ui_module_runtime::component::Session;

pub const MAX_RETIRED_TRANSACTIONS: usize = 256;

pub struct NavigationHost {
    pub session: Session,
    pub target: Option<Entry>,
    pub context: String,
    pub input_open: bool,
    receipts: HashMap<String, Applied>,
    retired: HashSet<String>,
    retired_order: VecDeque<String>,
    failures: HashMap<String, (Transaction, String)>,
    offer: Option<RecoveryApplied>,
}
impl NavigationHost {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            target: None,
            context: String::new(),
            input_open: false,
            receipts: HashMap::new(),
            retired: HashSet::new(),
            retired_order: VecDeque::new(),
            failures: HashMap::new(),
            offer: None,
        }
    }
    fn insert_retired(&mut self, id: String) {
        if self.retired.insert(id.clone()) {
            self.retired_order.push_back(id);
            while self.retired_order.len() > MAX_RETIRED_TRANSACTIONS {
                if let Some(evicted) = self.retired_order.pop_front() {
                    self.retired.remove(&evicted);
                }
            }
        }
    }
    /// New nav instance, same Host: unresolved receipts survive the rebind.
    pub fn rebind(&mut self, session: Session) {
        self.session = session;
        self.input_open = false;
        self.offer = None;
    }
    pub fn restore(
        &mut self,
        offer: Offer,
        publish: impl FnOnce(Option<&Entry>) -> Result<String>,
    ) -> Result<RecoveryApplied> {
        ensure!(offer.session == self.session, "old recovery session");
        self.input_open = false;
        let context = publish(offer.history.current())?;
        ensure!(!context.is_empty(), "context publication missing");
        self.target = offer.history.current().cloned();
        self.context = context;
        let applied = RecoveryApplied {
            offer,
            context: self.context.clone(),
            target: self.target.clone(),
        };
        self.offer = Some(applied.clone());
        Ok(applied)
    }
    pub fn ready(&mut self, applied: &RecoveryApplied) -> Result<()> {
        // A late old-session reply must not mutate the valid new session.
        ensure!(
            applied.offer.session == self.session && self.offer.as_ref() == Some(applied),
            "stale recovery ready"
        );
        if self.target != applied.target || self.context != applied.context {
            self.input_open = false;
            anyhow::bail!("actual context changed");
        }
        self.input_open = true;
        Ok(())
    }
    pub fn execute(
        &mut self,
        tx: Transaction,
        publish: impl FnOnce(Option<&Entry>) -> Result<String>,
    ) -> Result<Applied> {
        ensure!(
            tx.session == self.session && self.input_open,
            "old execution session or recovery gate"
        );
        if let Some(old) = self.receipts.get(&tx.navigation_id) {
            ensure!(old.transaction == tx, "same ID/different payload");
            return Ok(old.clone());
        }
        if let Some((original, error)) = self.failures.get(&tx.navigation_id) {
            ensure!(*original == tx, "same ID/different payload");
            anyhow::bail!("{error}");
        }
        ensure!(
            !self.retired.contains(&tx.navigation_id),
            "retired transaction replay"
        );
        ensure!(
            self.receipts.is_empty() && self.failures.is_empty(),
            "previous execution is not reconciled"
        );
        let context = match publish(tx.target.as_ref()).and_then(|context| {
            ensure!(!context.is_empty(), "context publication missing");
            Ok(context)
        }) {
            Ok(context) => context,
            Err(error) => {
                self.failures
                    .insert(tx.navigation_id.clone(), (tx, error.to_string()));
                return Err(error);
            }
        };
        self.target = tx.target.clone();
        self.context = context.clone();
        self.offer = None;
        let applied = Applied {
            target: self.target.clone(),
            transaction: tx,
            context,
        };
        self.receipts
            .insert(applied.transaction.navigation_id.clone(), applied.clone());
        Ok(applied)
    }
    pub fn receipt(&self, tx: &Transaction) -> Result<Applied> {
        let receipt = self
            .receipts
            .get(&tx.navigation_id)
            .ok_or_else(|| anyhow::anyhow!("execution result unknown; receipt unavailable"))?;
        ensure!(receipt.transaction == *tx, "receipt payload mismatch");
        Ok(receipt.clone())
    }
    pub fn failed_receipt(&self, tx: &Transaction) -> Result<String> {
        let (original, error) = self
            .failures
            .get(&tx.navigation_id)
            .ok_or_else(|| anyhow::anyhow!("no known failed result"))?;
        ensure!(original == tx, "failure payload mismatch");
        Ok(error.clone())
    }
    pub fn aborted(&mut self, tx: &Transaction) -> Result<()> {
        self.failed_receipt(tx)?;
        self.failures.remove(&tx.navigation_id);
        self.insert_retired(tx.navigation_id.clone());
        Ok(())
    }
    pub fn committed(&mut self, tx: &Transaction, h: &History) -> Result<()> {
        ensure!(
            h.history_id == tx.history_id
                && h.last_commit_id.as_ref() == Some(&tx.navigation_id)
                && h.revision
                    == tx
                        .base_revision
                        .checked_add(1)
                        .ok_or_else(|| anyhow::anyhow!("revision exhausted"))?
                && h.current() == tx.target.as_ref(),
            "unrelated commit confirmation"
        );
        if let Some(old) = self.receipts.get(&tx.navigation_id) {
            ensure!(old.transaction == *tx, "receipt mismatch");
        }
        self.receipts.remove(&tx.navigation_id);
        self.insert_retired(tx.navigation_id.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_transactions_evict_fifo_at_capacity() {
        let session = Session {
            module: "navigation".into(),
            instance: 1,
            host_epoch: 1,
        };
        let mut host = NavigationHost::new(session);
        for i in 0..(MAX_RETIRED_TRANSACTIONS + 5) {
            host.insert_retired(format!("tx-{i}"));
        }
        assert_eq!(host.retired.len(), MAX_RETIRED_TRANSACTIONS);
        assert_eq!(host.retired_order.len(), MAX_RETIRED_TRANSACTIONS);
        assert!(!host.retired.contains("tx-0"));
        assert!(!host.retired.contains("tx-4"));
        assert!(host.retired.contains("tx-5"));
        assert!(
            host.retired
                .contains(&format!("tx-{}", MAX_RETIRED_TRANSACTIONS + 4))
        );
    }
}
