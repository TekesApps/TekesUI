# Navigation Content recovery on the Combo17 prefix

Refs #195, original #63/#61/#14/#123. This is a source-only incremental recovery
against fe4851cc269d517e3b60387ebef4c45d646205ad, not a new component design.
Original Tabs labels/real retained panels, Breadcrumb string paths, Command's
original TextField/TextSurface query and ordinary Link string/action policy
remain. No destination or automatic browser action is added.

The private Menu header front door now takes model, labels, MenuRenderContext,
header size and paint callback (six arguments including self). FilteredMenuView
constructs that exact existing context. It does not restore the diagnosed
13-argument front door or add a lint allowance. Optional header geometry, one
opaque popup Presence and rows clipping remain the original shared behavior.
Ordinary Combo/Menu rendering passes no header; all existing Combo/Text/Menu
prepared-action, mount, generation and native authorization paths stay.

Only header-owned support is recovered from the saved deferred package.
MenuPreparedKind/Action/State, Select prepare/apply/replace helpers and tests,
and Button prepare_content_press are deferred with #199, not made prerequisites
of this navigation increment. No Selection/Search/Collections body is included.

TabsView is the original block SHA256
2e1cd152e35db8971cbfde157f96a4aef92cc94984f6bb8c22a22b785acbf3a7.
Its measured lanes, custom passive labels, real Panel Content, active subtree,
marked-text selection barrier and immutable action identities remain.

Prepared verification reuses the original seven consumer targets (51 ordinary
cases), two original Tabs wire tests, additive protocol/native data tests and
strict checks. Necessary header GPU/image checks follow calibration. This
candidate has not run Rust, GPU, IPC child, native, physical IME or VoiceOver.
Historical receipts retain their input identities. The original build helper,
serial shared cache and an explicit frozen-source allocation remain required.

The corrected active prefix removed its obsolete header None. This navigation
increment restores None only when the runtime regains its optional header. The
saved 0511 deferred reapply plus daf87f caller followup reconstructs the same
header source, with three real calls and the definition all taking 11 arguments.
