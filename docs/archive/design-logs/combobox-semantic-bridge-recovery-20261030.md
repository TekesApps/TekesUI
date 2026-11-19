# ComboBox matched-schema bridge recovery

Scope: [#194](https://github.com/TekesApps/TekesUI/issues/194) dependency ③, recovering the finite original [ComboBoxContent design](combobox-content-20261024.md) / historical #116 bridge. The closed #90/#116 and their old receipts remain unchanged. This document does not accept the ComboBox adapter, prepared editor/menu seams, GPU/physical IME or VoiceOver.

## Ownership and baseline

MAIN and the historical gallery-design-baseline tree are read-only for this domain. The handoff records MAIN `4815c5abca796820f4d906858ea42c7dceb8e690` plus actual file hashes, and historical `4d09a57282db67519bc89b6a05a2bcdc6ab4828a` plus the actual historical WIP hashes. No old shared file is substituted for a current file. The new patch is built as specific hunks over MAIN, retaining the already delivered ProgressIndicator, Image, Splitter, Link and Dialog lifecycle changes.

The ComboBox subject owner supplies ContentRole::ComboBox and ComboBoxContent, plus the editor/menu prepared seams allocated to their owners. This bridge supplies only wire/runtime/platform semantics and its dedicated test witnesses. The separate five-selection-role owner and the integration executor own the final role union; the bridge adds ComboBox without copying their enums. Current AppHost window_accessibility.rs already consumes the platform ContentAccessibilitySnapshot and native key remapping, so no historical app-host/accessibility.rs is added.

## Finite contract

One original TextFieldContent/TextSurface owns the editable single-line ComboBox draft. Role projection preserves the original binding, UTF-16 text/selection/marked state and shaped layout. Popup option parts retain their existing role and parents; they do not become another editor. The native role is NSAccessibilityComboBoxRole, with the existing field text/selection/layout selectors, focus and popup-expanded operations routed through original node actions. Disabled/read-only admission and stale native objects keep the existing selector rules.

Wire SemanticRole::ComboBox is an additive matched-schema role. Outer VERSION remains 2; no old role encoding changes or protocol negotiation is added. An older complete Reply decoder rejects the whole reply when the new root or part role follows a valid prefix. The frozen pre-ComboBox decoder is a capture of the actual current complete schema, including ProgressIndicator, Image, Splitter and Link, not the historical pre-ComboBox role set. Only its local module path is adjusted. If the integration executor admits other assigned role additions first, it must merge those exact additive enum variants into the pre-ComboBox baseline too; ComboBox itself must remain absent. This does not imply old-peer interoperability.

Receiver and platform projections reject multiline ComboBox layouts instead of relabeling them TextArea. Root and virtual-part text layouts retain the existing owner/binding checks. No new editor or failure reducer is added.

## Original rejection and publication behavior

Historical Combo snapshot, publication and ProcessContainer failure tests are recovered with their behavior assertions retained. They enter the real snapshot/action, RemoteSemantics::update/revoke and ProcessContainer::accept_content/content_failure paths. The historical retired-process helper is absent from the current owner; the disconnect request-failure case now enters the existing content_failure boundary directly. This tests the actual request-owner failure fence, not a real OS process disconnect. Hide/deactivate/cancel/unmount, malformed reply, old decoder failure and neighboring Composer marked text/focus/binding preservation remain explicit tests.

The original publication test requires failed multi-entry allocation not to partially commit the monotonic ID allocator. Current update mutated self.next during construction. The recovery uses one local next counter and commits it alongside the complete new entries only after success; on failure the existing dirty fence still hides old entries/callbacks/layouts. This is the original checked-capacity failure contract, not a new speculative concurrency mechanism.

The historical test-only ComboOwner witness forwards to one real TextFieldContent and the original scalar ComboBox model; it has no popup/editor state machine. Runtime snapshot tests reuse the existing public consumer CoreText backend. The platform public test additionally uses the recovered real ComboBoxContent, so it depends on the subject/editor/menu batch. A multiline original editor rejection test uses the same finite witness.

The original #116 native SDK object contract is isolated in combo_native_contract.rs and an explicit combo_bridge_native example. It retains the original enabled/read-only, role/subrole, UTF-16 range, layout, setter/action and stale-object assertions. The obsolete SearchField comparison is reduced to the actual ComboBox empty-subrole assertion; the current Shared readiness boundary is established before native dispatch. It creates one unshown native window and requires a separate native allocation. No ordinary data-test command runs this native program. Its historical count/evidence is not a current pass.

## Verification and integration

The handoff contains a complete source/test/design patch, MAIN-before and candidate-after hashes, historical input references, patch replay/static checks and exact scoped commands. Static syntax/format and patch replay are distinct from compilation. No Cargo, GPU or native program runs during this isolated preparation; the earlier #5 native window is already released and its failure/fix stays in its own frozen bundle.

After the integration executor unions the current roles and admits the three prepared dependencies, use the existing registered SSD review cache and build_lock/environment for the protocol combo_schema test, runtime combo snapshot/failure/publication tests, platform ComboBox public projection/UTF-16 tests and scoped strict checks. Compile the explicit native example without launching it. Check current AppHost as a public projection consumer. The original subject owner runs the original ComboBoxContent public consumer/GPU comparisons in its own accepted scope. A new target/cache, source rebind or native run is not authorized by this patch.

Unverified: all current Rust behavior/strict for this batch, the recovered native SDK probe, real process disconnect, externally observed AX notifications, shown native window/physical IME/VoiceOver/performance and global CI/publication. Global metadata and role-union integration remain with the unique integration executor. Do not close #194 from this bridge dependency alone.
