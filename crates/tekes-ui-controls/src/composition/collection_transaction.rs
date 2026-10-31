//! Shared original Table transaction tree access. Immutable path planning precedes
//! indexed mutable access, so unrelated Unknown siblings are never mutably visited.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CollectionTransactionError {
    Composition(CompositionError),
    UnknownTransactionHooks { child: ComponentId },
    Overflow,
}
impl From<CompositionError> for CollectionTransactionError {
    fn from(e: CompositionError) -> Self {
        Self::Composition(e)
    }
}
pub(super) fn transaction_outside_count(
    c: &Content,
    target: ComponentId,
) -> Result<usize, CollectionTransactionError> {
    let mut n = 1usize;
    if c.id() != target {
        for child in c.component.children() {
            n = n
                .checked_add(transaction_outside_count(child, target)?)
                .ok_or(CollectionTransactionError::Overflow)?;
        }
    }
    Ok(n)
}
pub(super) fn transaction_access_path(
    c: &Content,
    target: ComponentId,
    path: &mut Vec<usize>,
) -> Result<bool, CollectionTransactionError> {
    if c.id() == target {
        return Ok(true);
    }
    for (index, child) in c.component.children().iter().enumerate() {
        path.push(index);
        if transaction_access_path(child, target, path)? {
            if c.component.table_transaction_compatibility()
                != TableTransactionCompatibility::StableHooks
                && c.component::<ListContent>().is_none()
            {
                return Err(CollectionTransactionError::UnknownTransactionHooks { child: c.id() });
            }
            return Ok(true);
        }
        path.pop();
    }
    Ok(false)
}
pub(super) fn transaction_at_path_mut<'a>(
    c: &'a mut Content,
    path: &[usize],
) -> Option<&'a mut Content> {
    if let Some((index, rest)) = path.split_first() {
        transaction_at_path_mut(c.component.children_mut()?.get_mut(*index)?, rest)
    } else {
        Some(c)
    }
}
pub(super) fn transaction_outside(
    c: &Content,
    target: ComponentId,
    ids: &mut BTreeSet<ComponentId>,
) -> Result<(), CompositionError> {
    if !ids.insert(c.id()) {
        return Err(CompositionError::DuplicateId(c.id()));
    }
    if c.id() != target {
        c.component.validate()?;
        for child in c.component.children() {
            transaction_outside(child, target, ids)?;
        }
    }
    Ok(())
}
pub(super) fn transaction_declared(c: &Content, target: ComponentId) -> bool {
    if let Some(s) = c.component::<ScrollContent>()
        && s.reading_document_version().is_some()
        && s.children.iter().any(|c| c.find(target).is_some())
    {
        return true;
    }
    c.component
        .children()
        .iter()
        .any(|c| transaction_declared(c, target))
}
