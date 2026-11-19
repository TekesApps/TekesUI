# Modal AX scope and closed-window lifecycle

Scope: [Issue #5](https://github.com/TekesApps/TekesUI/issues/5), limited to the existing macOS accessibility bridge and its original ContentHost publication path. This is the next batch after the separately verified ProgressIndicator delivery. It changes no component focus traversal, input routing, Menu/Disclosure consumer, runtime/protocol or global metadata.

## Current contracts and evidence boundary

ContentHost already limits semantics to its active modal/overlay scope. DialogContent owns the original open/close model; show_dialog/close_dialog preserve the invoker mount and retained child state. ContentAccessibilitySnapshot and WindowAccessibilityPublication already carry that original tree to Window; native elements route actions through the original publication and handler. Existing source tests cover Dialog model/actions, modal API routing and caller identity. These are not evidence that the new native probe has run.

The broader existing schema covers Button and composed icon actions, selection roles, TextInput, List, Menu/MenuButton and Dialog, with name/value/disabled/selected/expanded/invalid/required metadata and real original actions. See composition.rs, composition/button.rs, composition/adapters.rs, composition/menu.rs and platform content_accessibility.rs. Native role/name/value/action coverage and actual VoiceOver acceptance for all these families remain separate from this limited batch.

## Concrete failure and smallest repair

At the inspected MAIN source, AppDelegate.windowWillClose set Shared.closed but retained the whole-batch publication, AX nodes, shaped text layouts and cached native elements. Shared.finish revoked the publication only later. Most native node/children queries checked closed, but TekesView.accessibilityFocusedUIElement directly searched its cached focused node without that guard. A client retaining the view could therefore receive its old focused element after actual window closure, and a retained publication clone could remain current until application teardown. This is a direct close path, not a proposed race.

Shared.revoke_accessibility now revokes and removes the existing publication and clears the three AX stores. Both windowWillClose and finish call this one teardown responsibility after marking closed. The view's focused-element query also checks closed and preserves its existing self fallback. No input/editor state is reset, no control state machine is copied, and no extra notifications are posted after closure.

## Regression paths

`tests/ax_modal_projection.rs` uses the original DialogContent, ListContent, Button, SwitchContent, ContentHost and platform publication adapter. It checks hidden closed Dialog nodes, modal-only projection and background action rejection, old geometry authority revocation, original Switch action/state, invoker restoration, reopen identity/state and inactive-host withdrawal. A local text backend supplies only measurement/paint services for this data test; the fixture owns no replacement Dialog or focus model.

`examples/ax_modal_lifecycle.rs` is an explicit native example, so ordinary test execution cannot accidentally launch it. In an allocated native window it uses public App/Window, publishes the original Host snapshot through the production Window setter, finds the real exported children, and invokes three native Press selectors. The ordinary handler uses publication.dispatch for the original actions and original host show/close callbacks. It checks modal role, hidden background, selection value, restored native focus, then calls Window.close. Before App.run finishes it requires saved publication revocation, no exported children, view focus fallback and inert retained nodes; it repeats retained-client queries after finish. It creates one real window and submits no GPU work or physical input. It does not verify actual VoiceOver speech or externally observed AX notifications.

The existing notification path compares structure/focus/value/selection in TekesView.accessibility_nodes_changed and publishes native notifications. This batch neither changes that algorithm nor claims an external notification observer has verified it. Actual notification delivery, VoiceOver, physical IME, GPU pixels and performance remain unverified.

## Identity and execution plan

The isolated checkout remains at `8a6a7633eec88af86353ad55da1aa90608faf69d` plus preserved WIP. The three production files and platform manifest were compared byte-for-byte with the inspected MAIN inputs before editing; current integration labels do not replace those hashes. The handoff records MAIN-before and candidate-after identities for every allowed file. Prior Progress patches and receipts stay untouched.

No Cargo, native executable, GPU, desktop input or system setting changes ran in preparation. Rustfmt syntax/format, patch applicability and pure-Rust source checks are recorded separately from unexecuted behavioral tests. The finite handoff runner prepares the data test, scoped strict checks for feature/default builds and the native example binary only after coordinator integration and an allocated shared Rust window. Native execution needs a separate allocated window through the existing desktop guard. Do not create a target/cache or change bindings to test this isolated source.

Accept the source batch after the new data behavior and scoped strict checks pass; keep actual native-selector acceptance and parent Issue #5 open until their corresponding allocated execution succeeds. A missing native window does not justify new control behavior or replacing VoiceOver with programmatic AX.

## Integration repair verification — 2026-10-30

The original `311ceb4d…` eight-file patch and original assertions were retained. A real feature Clippy failure in the native example (E0277/E0308) was returned to the original AX domain; it supplied only the two retained-object borrowing fixes, patch SHA-256 `4a5b54e79f91054296b9eaa27442bbd50db9ea809af1792a1c39df4d21782597`. The repaired example SHA-256 is `afe0686f9b5007dc9147c9c78d37e23bcb7583f07d30f5dd0ab04999cfd0abd4`.

Actual repair inputs were MAIN HEAD `29b62511d2797e0389cc4ed6aae7f76904093a31` plus those exact source patches. Feature strict Clippy (lib/test/example), default lib strict Clippy, and the content-accessibility example build all passed. The earlier two actual passing data tests were reused after byte comparisons confirmed unchanged test/fixture/production/dependency inputs; the example-only repair does not affect those tests. Original failure logs remain at SSD `artifacts/delivery/integration-executor-20261030-07/ax05/`, and repaired input comparisons/commands/executable copies are at `artifacts/delivery/integration-executor-20261030-08/ax-repair01/`.

The compiled native example has not been executed. External AX notifications/selectors, VoiceOver, physical IME, native window behavior and performance remain unverified; no native acceptance or maturity value was promoted. The existing composition audit only updates the three changed platform source hashes and three real shared.rs line anchors.


## Allocated native verification — 2026-10-30 round04

The fixture-only timing repair was admitted as MAIN
`8c5c3bab5b17c5ddc1a04a5226216a27d28e3f1a`; example SHA256
`78eb9073ff133838c538b0b14a3bcbca85eb804addd0b889c1b4ba56532cf7cf`.
Scoped example Clippy with `-D warnings` and Cargo JSON build passed in the
allocated original helper/cache environment. The actual Cargo program was
archived with SHA256
`afe01421d17a30b94ed1c11b5ff01959cc16264b69a0fea615e4efb44c8188d5`.
Earlier data/production/default-library passes are reused only for unchanged
inputs; this new fixture was independently compiled and run.

One separately allocated native attempt exited 0 without timeout or detected
user-input interruption. All four fixture screen queries reported actual
external display ID2; the named probe CG frame was X2072/Y239/800x632. Original
modal role/value/focus, three native Press selectors, close revocation/cache/
focus/stale-client assertions and after-App.run retained-client assertions all
completed. `AX_MODAL_CLOSE_COMPLETED` appeared once before the original final
success marker. This is finite same-process native selector/lifecycle evidence.

Raw result, independent final build binding, stdout/stderr, actual window/display
observations and immediate release receipt are preserved under
`/Volumes/HP P900/DEV/TekesUI/artifacts/issue-5/native-round04-once/`.
`result.json` SHA256 is
`b782d920d61ed09fa603788e69ebaf52e926b29f9eed95f75447e588c9b46d4e`.
Owned PID83764 was reaped. Both original guards released, and their existing
advisory locks could be acquired and immediately released after context exit.
No retry, Cargo, settings, MAIN/Git or metadata write occurred in the native
window. Earlier role/subrole abort and round03 timeout evidence are unchanged.
The resource/source lease ended before offline evidence analysis.

External AX notifications, physical IME, VoiceOver, GPU pixel acceptance,
performance and other Issue #5 acceptance criteria remain unverified. Keep the
Issue open; this result does not replace those acceptance categories.


## #205：外部 Layout 通知与分阶段验收（2026-11-05）

本批对应 [Issue #205](https://github.com/TekesApps/TekesUI/issues/205)。原设计的真实原生 owner 要求不变；以下记录具体实现和有限验证。

结构快照发布现在通过 `NSAccessibilityPostNotificationWithUserInfo` 发送 Layout，
`NSAccessibilityUIElementsKey` 携带当前真实原生根元素数组；发送对象为
`NSAccessibilityUnignoredAncestor(TekesView)` 实际返回的对象。没有返回对象时不制造
替代对象或通知。Focus、Value、SelectedText、缓存与关闭撤权路径保持原行为。
外部 Layout 订阅须在首个动作前绑定该实际对象；不能将 Focus 的 NSApp 路由推定为
Layout 的路由，也不能从已收到的事件反选期望来源。

新增独立 `ax205_staged_acceptance` example，复用原 `ax_modal_fixture` 和公共发布接口。
真实动作及 snapshot 提交保持同步；窗口/布局祖先和原生属性取证移到已有 `WindowWake`
主队列阶段，全部原断言成功后才生成阶段 marker，避免取证嵌套在外部 AXPress 回调内。
它需要协调方分配的原生资源窗口及 `TEKESUI_AX205_CHANNEL`；外部控制器按原握手执行
三次 live Press、关闭时保持进程存活、旧引用请求、同 Host 无副作用检查和单独 FINISH。
这不是普通 unattended Cargo test，不能脱离资源分配直接启动。

受控实际证据位于 `artifacts/issue-205/layout-ancestor-20261105-r1/actual-run-001/`：
冻结程序 `537af4f7f57beda23ae58252569581519f0e665bf7ea07c410efa3b6917429d0`
完成 46 步，source-aligned oracle 25/25 通过；包含真实 Layout/Focus/Value、三次 live
AXPress、进程存活下的关闭撤权和同 Host 不变、最终正常退出。stale 请求 raw AXError
与实际无副作用/原生拒绝证据分别判断，不能把 raw0 当作 Host 接受。

该程序来自 MAIN 的冻结 WIP overlay，包含未公开的 IME 诊断源差异；不能将其签成
本次精简公共提交的实测。公共候选仅接入 Layout 修改、这个独立 example 和本文档，
不包含 app/shared/view 的 IME 诊断改动。独立差量审核确认这些诊断没有改变本次
通知、动作、Host 与关闭契约，因此复用上述功能证据；这不建立公共程序的实际运行身份。
公共精确 tree 仍须通过原 required CI，其中包含 content-accessibility 全 targets 严格检查
及本 example 的显式编译链接。原默认发送、payload-only 和错误订阅条件的失败记录均保留。
25/25 是此分阶段 AX 契约的有限通过，不等于整个 #205/#5、VoiceOver、物理 IME、GPU
视觉、性能或全部外部窗口操作通过。外部 V7 控制器和原始证据当前仍在受管理 artifacts，
本批没有把本机 guard/权限配置复制成可任意执行的公共脚本。
