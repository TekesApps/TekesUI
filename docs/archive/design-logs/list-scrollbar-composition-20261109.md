# List scrolling composition (Issue #251)

The public List is **ScrollArea + Items + Model**. It does not include Panel and has no dependency on AppHostContainer.

## Public construction

Keep `ListContent::new(name, children)` and its public `rows` field compatible. Add the thin public factory:

```rust
Content::new(
    scroll_id,
    ListContent::scrollable(name, items_id, children).scrollbar(true),
)
```

`scroll_id` is the normal caller-owned outer Content identity. `items_id` is an explicit distinct caller-owned Content identity for the original ListContent Items node. Existing ContentHost validation detects duplicate IDs. The factory returns the original ScrollContent, containing `Content::new(items_id, ListContent::new(name, children))`; no new global ID allocator or parallel scrolling component is introduced.

The caller's existing Items and Model describe groups, ordering and selection. Group headers and grouped items remain original Content subtrees and keep their original IDs, lifetime, controls and reducer state. The factory introduces no duplicate selection reducer. It does not weaken the KeyedSelectableList passive-display or transactional update contracts and does not change legacy SelectableList struct literals. A normal property editor remains an editor rather than becoming an implicit row-selection action.

## Compatibility defaults

Existing `ScrollContent::new` preserves its effective no-bar presentation and geometry by hiding bars on that supplied original ScrollArea. Standalone `ScrollArea::default` is unchanged. The new List factory explicitly enables bars by default. `.scrollbar(false)` changes only appearance and pointer targets, never axis policy, wheel or reveal permissions.

## Scrollbar configuration and original owner

`ScrollContent::scrollbar(bool)` is a consuming builder that delegates appearance/hit availability to its existing ScrollArea. `true` shows a bar only on an overflowing enabled axis. `false` removes the bar and its pointer targets while preserving scrolling range, wheel and programmatic/focus reveal. All sections share that one original ScrollArea.

For the List factory's requested bars, an overflowing thumb is painted and accepts a new pointer Down while the pointer is inside the original ScrollArea viewport. Leaving the viewport hides it; an active thumb remains painted and captured until the original pointer-up/cancel path ends the drag. Pointer Up's own position updates hover, so a final Move callback is not required before release. `.scrollbar(false)` remains a permanent hide. The overflow gutter and child clip stay stable while the thumb is hidden, so entering/leaving does not reflow List items.

ScrollContent gains original ScrollArea scrollbar paint and pointer input; it retains its existing wheel, clipping, reading and reveal responsibilities. There is no new momentum, offset, drag-physics or animation reducer.

A requested overflowing bar's layout, hit track and paint track derive from the same original viewport geometry and proposal scrollbar metrics. Its exclusive child-clip band remains reserved while hover-hidden so full-width child controls cannot steal its Down when it appears; `.scrollbar(false)` and no-overflow reserve no band. Reflow/layout must account for the receiving width and preserve the existing focus bleed and ancestor clipping contract.

## Input and transaction behavior

Pointer Down uses existing pure ScrollArea.prepare_pointer_down and its immutable ScrollPointerDecision. Commit is fenced through the original prepared action mechanism, preserving keyboard focus and composing input. Only an admitted actual thumb/page action updates offsets, stops reading-follow, fences old momentum and captures the pointer. Up/cancel/hidden/layout changes terminate the original pointer gesture. Rendering/layout acknowledge current child/IME/reveal geometry through the existing Host pipeline.

The real ScrollContent node remains visible to the original typed reveal/reading/prepared lifecycle machinery; no opaque List-owned alternate ScrollArea bypasses those contracts. Scrollbar painting uses the existing proposal geometry and Theme roles, never AppHost-specific colors.

## Consumer scope

Gallery AppHost navigation and property grouped Items adopt the List factory and remove their directly constructed redundant external ScrollContent wrappers. Other non-List scrolling documents continue using ScrollContent. AppHost presentation layout and Others containers implement no List scroll state.

## Verification

Use real ContentHost paths to verify: explicit List/Items identities; one ScrollContent owner for multiple groups; overflow/no-overflow and true/false paint/hit behavior; pointer entry/exit visibility over an item; active thumb drag visibility outside the viewport through release; actual thumb drag and page scrolling with no item activation; wheel and reveal with bars hidden; preserved keyboard focus/marked input; pure/fenced preparation; cancellation; Theme and clipping; existing reading/reveal regressions affected by reserved viewport width. Retain unrelated WIP and reuse unchanged passed evidence. Native GUI acceptance is recorded separately from source/unit evidence.

## AppHost Gallery spacing

Gallery explicitly gives all four AppHost containers zero outer content padding and assigns one `Spacing::Loose` (16pt) document padding to their original ScrollContent. Generic container and ScrollContent defaults remain compatible. The inner gutter is max(requested padding, original focus bleed); visible scrollbar exclusion reserves only max(0, bar band + focus bleed - gutter), so an existing 16pt margin can hold both the original focus ring and scrollbar without an extra asymmetric inset.

Navigation uses the existing Gallery `ControlSize::RowHeight` and `Spacing::Micro` row metrics through an explicit ListView opt-in; default ordinary List row metrics remain unchanged. Existing section headers and selectable models retain their IDs. Each header plus its items is an original Stack with Regular gap, and sections share an original Stack with Micro gap; the Workspace heading remains present.

`layout.scroll_area.content_padding` 12pt is the existing TextSurface Field interior contract; Content text uses its existing zero interior padding. AppHost pane boundary spacing is a separate explicit `Spacing::Loose`16 request. Field padding and label insets are unchanged.

## 2026-11-12: shared List resting rows

An unselected row with no pointer interaction has no decorative background. This is the shared List behavior, including Gallery navigation, rather than a consumer color override. Hover and press use the existing parent Surface action-quiet state tokens; selected rows retain selection fill and disabled rows retain disabled foreground. Keyboard focus, clipping, original geometry and the hover/drag-only scrollbar contract are unchanged. Segmented controls keep their own resting chrome. No theme color values are added for this correction.


## #274 master-detail consumer（2026-11-13，用户已确认）

本轮将 Gallery master-detail 明确纳入标准 List 消费范围：左侧80文档由 `ListContent::scrollable` 直接组合原 ScrollContent/Items/keyed selectable List，保留稳定文档key和全部挂载行，不增加冗余外层滚动器或虚拟化。Gallery 消费原选择事件/selectedId更新业务文档与右侧既有 TextSurface，不再维护独立方向键选择器或手绘不可交互thumb。外层Tab继续交接至detail；native key投影共享原Gallery映射，不复制选择或输入owner。

适用验收是原thumb拖动改变同一offset且零行激活，wheel/reveal/clip来自同一几何，selection/detail一致、hover离开恢复、Home/End/Tab保持，以及非precise行单位仅在原ScrollArea换算一次。CPU、原生及公开交付分别回填#274；本段记录已批准合同，不预先宣称实现/原生验收完成。

键盘导航 reveal 由公共 ContentHost 接收原 keyed List 已消费的 Up/Down/Home/End 请求；同帧按键合并到最后一个已注册 row，正常布局后调用原 `plan_host_reveal` / `commit_host_reveal`，不在 Gallery 计算 offset。失焦、失活、卸载及后续 pointer/wheel 撤销待处理请求。`navigation_reveal_result()` 保留最后一次完整 `Result<HostRevealOutcome, HostRevealError>`；一次拒绝不自动逐帧重试，也不回退私有滚动。合法空 clip 可能先撤销布局平移能力并返回 `UnsupportedLayout`，保留 planner 原原因，不重新归类。render 的正常布局错误仍通过原 CompositionError 返回；reveal 拒绝与绘制结果分别可观察。
