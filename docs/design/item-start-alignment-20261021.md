# Issues28/32 selected-axis item Start

Root owns generic library geometry; AgentKit owns ordinary-turn vs semantic-last-turn decisions,initial latest and follow deduplication. This increment uses the existing Axis,ScrollArea,ScrollGeometry,reading attachment/mount guard,ContentHost layout/text preflight and original motion/input ownership. No new registered component or token value.

`ScrollRevealPlacement::Start(extended::Axis::Vertical)` aligns the original target top to the actual visible content start plus explicit top margin. Horizontal document offset is retained. Start(Horizontal) selects the other existing axis;the default Nearest and existing Centered semantics do not change. Start moves an already-visible middle/tail or viewport-tall target; it is not nearest-to-visible. ScrollArea reports existing selected-axis document/policy clamp explicitly.

ContentHost derives each selected-axis clip inset from original ScrollGeometry and current ancestor clip. Offscreen or temporarily too-small nested clips may become usable when a parent reveals them. The existing same-tree endpoint preflight refines those plans up to ancestor_count+2 passes,checks original path/mount and requires usable final clipping. Only library scroll/reading metadata is saved/restored;editors,children and application components are never cloned. Intrinsic unreachable or oscillating clips reject;original malformed margins still reject. Application layout must be deterministic for unchanged inputs;its arbitrary layout side effects are not rolled back.

Original host frame or clip changes interrupt an in-flight plan before its old tick. An explicit retry uses new geometry. Original owner/version/mount/ABA validation,same-extent target-movement interruption,old momentum fencing,fresh wheel ownership,history intent,Reduced Motion and host/unclocked completion stay shared. No synthetic focus,click,IME callback or replacement editor.

Fractional user offsets exposed an existing shared reading-anchor round-trip issue. Displacement is now computed in f64 and differences within the pre-existing0.001 anchor clamp tolerance retain the offset bit-for-bit. This numerical restoration guard does not add a comfortable nearest tolerance to explicit Start.

Business usage:

```rust
let options = ScrollRevealOptions {
    placement: ScrollRevealPlacement::Start(Axis::Vertical),
    margins: Insets::ZERO,
    animated: true,
};
let result = host.reveal_scroll_reading_item(owner, &version, item, options, original_renderer)?;
// Request/render the successful current frame before native publication even
// when an accepted result has changed=false. Final-turn follow stays business-owned.
```

Evidence is split in design/reviews/item-start-alignment-20261021:public viewport and original mixed-tree API tests,actual GPU,actual token observations,native and isolated delivery receipts. The mixed consumer covers clipped and initially tiny/offscreen nested viewports,visible/tall targets,head/tail clamps,preedit and both selection directions/undo,ABA/stale/removal/replacement,ordinary/RM/unclocked completion,wheel interruption,new clip and same-extent movement with explicit recovery. No consumer offset compensation or borrowed editor focus.

Scoped functional/Gold7 regression does not establish matched ClientKit product-page appearance or independent AgentKit native acceptance. Old545/292f/10b5/c779 packages remain immutable. Performance is not measured;VoiceOver stays off unless a later required hearing check is explicitly arranged. No commit,push,tag or Issue closure in this increment.

Final producer evidence:25 item /11 viewport /426 controls /227 consumer tests;strict controls/Gallery/consumer/GPU/native Clippy;96 real GPU cases/480 frames (Start includes actual root clip),17 actual bounded token probes/6432cases (complete=false). Producer native f7ec0ff10948d4b28bee9c33e7e60efbaa2a626cd7166807854e5cbdd35dfcb6 has17 actual active U2790B@2 checkpoints:original pointer control,visible-middle Start,clip150/180 and same-extent interruption/recovery,narrow large type,RM/normal motion,manual browsing through unrelated render,original native input and undo/reverse selection/redo. Ordinary native X input is not physical IME;public/GPU preedit is separate. Initial unsupported AX-text-click/paste attempts are diagnostic only. Matched ClientKit page,isolated source/native and receiver acceptance remain separate.
