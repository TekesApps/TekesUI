//! Persistent values contain business keys only. Runtime requests retain the
//! current attachment and input intent; a saved value grants no authority.
use super::*;

/// Format 1 uses item-local logical points, not text/UTF-16 positions. Hosts
/// serialize these public values in their own storage format and report read
/// failures separately from absence. No ComponentId or attachment is persisted.
#[derive(Clone, Debug, PartialEq)]
pub struct ContentReadingCheckpoint {
    pub format: u32,
    pub document: String,
    pub revision: u64,
    pub position: ContentReadingPosition,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ContentReadingPosition {
    FollowingLatest,
    History {
        item: String,
        within: f32,
        screen_y: f32,
        /// Old loaded successors, then predecessors, in fallback priority.
        fallback: Vec<String>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentReadingCheckpointError {
    Reading(ReadingUpdateError),
    UnavailableOwner,
    UnlaidOut,
    InvalidItemKeys,
    InvalidCheckpoint,
    UnsupportedFormat,
    /// Declared in the current document, but not laid out. Load history and
    /// retry only while the original request is still current.
    ItemUnavailable(String),
    Superseded,
    ReadingTransactionInProgress,
}

/// Capture this before asynchronous storage/history work. Restore checks the
/// original attachment/revision, user intent and explicit navigation. A new
/// request must express a new authorized restore, not refresh a stale response.
#[derive(Debug)]
pub struct ContentReadingRestoreRequest {
    owner: ComponentId,
    version: ReadingDocumentVersion,
    intent: Rc<()>,
    reveal: u64,
}

impl ContentReadingCheckpoint {
    fn validate(&self) -> Result<(), ContentReadingCheckpointError> {
        use ContentReadingCheckpointError as E;
        if self.format != 1 {
            return Err(E::UnsupportedFormat);
        }
        if self.document.trim().is_empty() {
            return Err(E::InvalidCheckpoint);
        }
        if let ContentReadingPosition::History {
            item,
            within,
            screen_y,
            fallback,
        } = &self.position
        {
            let keys: BTreeSet<_> = std::iter::once(item).chain(fallback).collect();
            if item.trim().is_empty()
                || !within.is_finite()
                || *within < 0.0
                || !screen_y.is_finite()
                || fallback.iter().any(|s| s.trim().is_empty())
                || keys.len() != fallback.len() + 1
            {
                return Err(E::InvalidCheckpoint);
            }
        }
        Ok(())
    }
}

/// The mapping covers every declared item, including unloaded items. Omission
/// means deletion; presence without active nonempty layout means unavailable.
fn validate_keys(
    state: &ContentReadingState,
    keys: &[(String, ComponentId)],
) -> Result<(), ContentReadingCheckpointError> {
    if keys.len() != state.items.len()
        || keys
            .iter()
            .any(|(key, id)| key.trim().is_empty() || !state.items.contains(id))
        || keys
            .iter()
            .map(|(key, _)| key)
            .collect::<BTreeSet<_>>()
            .len()
            != keys.len()
        || keys.iter().map(|(_, id)| id).collect::<BTreeSet<_>>().len() != keys.len()
    {
        return Err(ContentReadingCheckpointError::InvalidItemKeys);
    }
    Ok(())
}

impl ContentHost {
    fn checkpoint_reader(
        &self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
    ) -> Result<&ScrollContent, ContentReadingCheckpointError> {
        use ContentReadingCheckpointError as E;
        if self.reading_transaction {
            return Err(E::ReadingTransactionInProgress);
        }
        let scroll = self
            .scope()
            .find_active(owner)
            .and_then(|n| n.component::<ScrollContent>())
            .ok_or(E::UnavailableOwner)?;
        let state = scroll
            .reading
            .as_ref()
            .ok_or(E::Reading(ReadingUpdateError::WrongDocument))?;
        state
            .version
            .validate_current(expected)
            .map_err(E::Reading)?;
        Ok(scroll)
    }

    /// Export only after a successful nonempty layout of this exact version.
    /// Keys belong to the host's business document, never to a process/tree.
    pub fn capture_scroll_reading_checkpoint(
        &self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
        keys: &[(String, ComponentId)],
    ) -> Result<ContentReadingCheckpoint, ContentReadingCheckpointError> {
        use ContentReadingCheckpointError as E;
        let scroll = self.checkpoint_reader(owner, expected)?;
        let state = scroll.reading.as_ref().unwrap();
        validate_keys(state, keys)?;
        if self.last_layout.is_none()
            || state.result.as_ref().is_none_or(|r| r.version != *expected)
            || state.frames.is_empty()
            || state.viewport.size.width <= 0.0
            || state.viewport.size.height <= 0.0
        {
            return Err(E::UnlaidOut);
        }
        let key = |id| {
            keys.iter()
                .find(|(_, k)| *k == id)
                .map(|(s, _)| s.clone())
                .ok_or(E::InvalidItemKeys)
        };
        let position = if state.following {
            ContentReadingPosition::FollowingLatest
        } else {
            let a = state.capture(&scroll.scroll).ok_or(E::UnlaidOut)?;
            ContentReadingPosition::History {
                item: key(a.item)?,
                within: a.within,
                screen_y: a.screen_y,
                fallback: a.fallback.into_iter().map(key).collect::<Result<_, _>>()?,
            }
        };
        let checkpoint = ContentReadingCheckpoint {
            format: 1,
            document: expected.key().into(),
            revision: expected.revision(),
            position,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    /// May be prepared before the first render, before starting storage IO.
    /// Layout alone does not retire a request. Routed user scroll/history,
    /// explicit follow/stop, navigation, restore or document updates do.
    pub fn prepare_scroll_reading_restore(
        &self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
    ) -> Result<ContentReadingRestoreRequest, ContentReadingCheckpointError> {
        let scroll = self.checkpoint_reader(owner, expected)?;
        Ok(ContentReadingRestoreRequest {
            owner,
            version: expected.clone(),
            intent: scroll.reading.as_ref().unwrap().intent.clone(),
            reveal: scroll.scroll.reveal_identity(),
        })
    }

    /// Validate all persistent values and mappings before changing state. The
    /// destination must have completed its first layout. Older revisions may
    /// restore into a newer document with the same key; future ones reject.
    ///
    /// A declared but unloaded target returns ItemUnavailable without installing
    /// fallback. An omitted (deleted) key resolves through saved neighbors then
    /// the first loaded current item. The next render performs SDK geometry,
    /// clamp and reflow before painting; input in between cancels the restore.
    pub fn restore_scroll_reading_checkpoint(
        &mut self,
        request: &ContentReadingRestoreRequest,
        checkpoint: &ContentReadingCheckpoint,
        keys: &[(String, ComponentId)],
    ) -> Result<(), ContentReadingCheckpointError> {
        use ContentReadingCheckpointError as E;
        let scroll = self.checkpoint_reader(request.owner, &request.version)?;
        let state = scroll.reading.as_ref().unwrap();
        if !Rc::ptr_eq(&state.intent, &request.intent)
            || scroll.scroll.reveal_identity() != request.reveal
        {
            return Err(E::Superseded);
        }
        checkpoint.validate()?;
        if checkpoint.document != state.version.key() {
            return Err(E::Reading(ReadingUpdateError::WrongDocument));
        }
        if checkpoint.revision > state.version.revision() {
            return Err(E::Reading(ReadingUpdateError::StaleRevision));
        }
        validate_keys(state, keys)?;
        if self.last_layout.is_none()
            || state
                .result
                .as_ref()
                .is_none_or(|r| r.version != request.version)
            || state.viewport.size.width <= 0.0
            || state.viewport.size.height <= 0.0
        {
            return Err(E::UnlaidOut);
        }
        let pending = match &checkpoint.position {
            ContentReadingPosition::FollowingLatest => None,
            ContentReadingPosition::History {
                item,
                within,
                screen_y,
                fallback,
            } => {
                let resolve = |key: &str| keys.iter().find(|(s, _)| s == key).map(|(_, id)| *id);
                // Omitted keys are deleted; declared keys with no frame are
                // unloaded. Never claim a fallback is the unavailable target.
                let candidate = resolve(item).map(|id| (item, id)).or_else(|| {
                    fallback.iter().find_map(|key| {
                        let id = resolve(key)?;
                        state
                            .frames
                            .iter()
                            .any(|(k, _)| *k == id)
                            .then_some((key, id))
                    })
                });
                let (key, id) = candidate
                    .or_else(|| {
                        state.frames.first().and_then(|(id, _)| {
                            keys.iter()
                                .find(|(_, k)| k == id)
                                .map(|(key, id)| (key, *id))
                        })
                    })
                    .ok_or(E::UnlaidOut)?;
                if !state.frames.iter().any(|(k, _)| *k == id) {
                    return Err(E::ItemUnavailable(key.clone()));
                }
                Some(Anchor {
                    item: id,
                    within: if key == item { *within } else { 0.0 },
                    screen_y: *screen_y,
                    fallback_used: key != item,
                    fallback: fallback
                        .iter()
                        .filter_map(|key| resolve(key))
                        .filter(|k| *k != id)
                        .collect(),
                })
            }
        };
        // No callbacks, IO or fallible work past this point. This only updates
        // original reading/scroll state, never focus, selection, IME or content.
        let scroll = self
            .root
            .find_mut(request.owner)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap();
        scroll.scroll.cancel();
        let state = scroll.reading.as_mut().unwrap();
        state.intent = Rc::new(());
        state.following = matches!(checkpoint.position, ContentReadingPosition::FollowingLatest);
        state.pending = pending;
        state.restore_offset = None;
        state.frames.clear();
        state.item_mounts.clear();
        state.explicit_reveal = None;
        state.momentum_fenced = true;
        state.user_input = false;
        state.result = None;
        self.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pure_value_boundary_rejects_corrupt_geometry_keys_and_future_format() {
        let original = ContentReadingCheckpoint {
            format: 1,
            document: "doc".into(),
            revision: 0,
            position: ContentReadingPosition::History {
                item: "a".into(),
                within: 3.5,
                screen_y: 0.0,
                fallback: vec!["b".into()],
            },
        };
        assert_eq!(original.validate(), Ok(()));
        for coordinate in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut value = original.clone();
            if let ContentReadingPosition::History { screen_y, .. } = &mut value.position {
                *screen_y = coordinate;
            }
            assert_eq!(
                value.validate(),
                Err(ContentReadingCheckpointError::InvalidCheckpoint)
            );
        }
        let mut value = original.clone();
        value.document = " \n".into();
        assert_eq!(
            value.validate(),
            Err(ContentReadingCheckpointError::InvalidCheckpoint)
        );
        let mut value = original.clone();
        value.format = 2;
        assert_eq!(
            value.validate(),
            Err(ContentReadingCheckpointError::UnsupportedFormat)
        );
        let mut value = original;
        if let ContentReadingPosition::History { fallback, .. } = &mut value.position {
            fallback.push("b".into());
        }
        assert_eq!(
            value.validate(),
            Err(ContentReadingCheckpointError::InvalidCheckpoint)
        );
    }
}
