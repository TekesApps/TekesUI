# Combo SetExpanded action closure

Refs #194; repair the observed E0599 on frozen7472178. This is an additive
repair against canonical823a54184f13acdf6852dca86745551a8e86e558bd17e53ee9839bd587199df3
(baseline0aec889754561a24b13976830fab30ade0c268a6). Current roles remain the
Combo-only17-role schema; no other role cohort, subject, state machine or host
fork is restored.

The original Combo already implements explicit true/open and false/close in its
prepared owner. Current shared try_prepared_ax already recognizes Combo expansion
and applies original focus/composition admission. The missing public variant
causes the real E0599. Native setters already dispatch SetExpanded and advertise
it for Combo; their Content adapter still rejects the action. The wire lacks the
verb and its two runtime directions lack mappings. Repair all five missing seams
without converting expansion to Press. No focus, readonly, editor/marked binding,
selection, stale mount, transaction or once-owner algorithm is changed.

The public enum lives in shared composition.rs. This package contains only its
exact additive hunk; the integration executor is the sole current shared-file
writer and admits it with the original shared owner, avoiding concurrent edits.
The remaining four production hunks belong to AX protocol/runtime/platform.
Do not edit action_preparation.rs, Combo body, Text pointer capture or Menu width
from this batch. Their separate fingerprints remain their owners' work.

The previous AX superset already had the three wire/runtime mapping hunks and
native mapping; use these narrow hunks once instead of importing its Tree/Table/
Search/Select or peer roles. Do not apply both action deltas twice. Old decoder
is the actual frozen canonical pre-Combo16-role decoder; current17-role checks
remain unchanged. New complete Input test proves old decoder rejects SetExpanded
with original target/part/binding while current true/false roundtrip at VERSION2.
Native data adapter and runtime tests use actual ComboBoxContent and repeated
true,true,false,false to distinguish explicit state from toggling Press. Runtime
also rejects stale text binding before the valid original-owner action. Existing
Combo marked/readonly/once-owner and neighboring composition tests stay intact.

Only source/static checks run here. Typecheck, behavior, strict and native are
not_run until integration admits the complete canonical source plus owner fixes
and allocates its original helper/cache window. Preserve original compiler red
logs. Keep #194 open until its complete acceptance and public delivery.
