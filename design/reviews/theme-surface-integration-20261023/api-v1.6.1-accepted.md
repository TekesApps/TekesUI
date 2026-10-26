# Theme / Surface consumption: Schema1.2, rules1.6.1

Design System owns the Theme definition schema, concrete definitions, shared color generation and validation, and component contracts. Theme definition schema1.0, default/blue/clay definition versions1.0 and generated format `tekesui.theme-instances/1` remain unchanged. Each concrete definition contains its four appearance/contrast modes. Rust `Theme` remains the mode selector; `Palette` selects the existing compiled BrandConfig and mode. No runtime JSON parser or per-component color derivation is introduced.

Surface Schema1.2 / rules1.6 add four passive status uses to the previous27. There are31 uses and188 valid use/state pairs per Surface. Seven canonical Surfaces and13 registered Palettes produce68,432 complete results. Python generates three preview definitions /15,792 results. The independent15-config audit includes two additional own-brand consumers and checks78,960 results; these are not15 Theme definitions. Every previous15,120 preview packet remains exactly unchanged.

## Identity, paint and state ownership

A container declares a canonical Surface name and paints that Surface's generated ground. Original Content context propagates the name. Explicit node overrides take precedence; clearing one resumes inheritance. Unknown names fail before mutation. Never infer identity from RGB: different named Surfaces may share the same ground and have different foreground profiles.

`resolve_surface(palette, surface, usage, state)` returns the entire `SurfaceColors` packet: background, text, icon, boundary, focus, marker and helper. Components select applicable states from their existing models and consume the generated packet. Consumers do not supply rest/hover/pressed color tables. Action text and icon channels include their corresponding hover and pressed foregrounds. Original interaction progress may interpolate complete generated endpoints; Theme owns no state machine.

The previous input/error/readonly, action, selection, check/toggle anatomy, range and frame contracts remain in effect. Necessary range boundaries differ from decorative frame separators. Labels outside an input consume the parent control-label packet; interior foreground and helper channels stay independent. Surface kind retains geometry, radius, elevation and clipping ownership. Historical rules1.5.3 details are recorded in [the previous contract](api-v1.5.3.md); its old versions/counts are historical.

## Link complete packet and anatomy

Rules1.6.1 is a validation-only correction. Link owns its generated background, optional boundary, text/underline and independent focus. Its original rest/hover/pressed/disabled set is unchanged. Bindings with non-null marker or helper are rejected in normal Python and Rust because Link has neither control part. Other shared profiles may retain these parts for uses that actually own them. Legal inverse backgrounds and optional Link boundaries remain supported; do not hide a painter omission by narrowing them away.

All31 uses,188 valid pairs and15,792 preview colors remain identical to1.6.0. The generated Rust role table is byte-identical. Only rules/source provenance changes. This patch contains no Progress use or rest-only profile extension. The public composition::LinkContent wraps the original Link model and LinkView. Its complete packet and content-fit geometry were independently sampled with the original public Composer fixture on source686d2f6e772e5c06216e391a86d50715800d9685b19c96320523cc5ff1de8bab:192 natural-page frames,24 focus causal frames,720 complete-state frames and288 actual Gallery frames. These results are source-bound offscreen GPU evidence; combined live adoption, native interaction and final whole-system acceptance remain separate.

## Passive status content

| Existing StatusBadge | Shared generated use |
| --- | --- |
| Info | content-status-info |
| Running | content-status-running |
| Succeeded | content-status-succeeded |
| Failed | content-status-failed |

Only `rest` and `disabled` are applicable. Default shared profiles inherit ordinary content: rest text/icon reference the original corresponding status role; disabled inherits content/disabled and its existing disabled-text. Ground must remain the parent `$surface`; boundary/focus/marker/helper are forbidden. The same profile reference/inheritance mechanism supports new Surfaces and valid text/icon channel overrides. Components must consume the packet rather than hardcode a role from the status enum. Status colors retain their existing semantics independently of brand accent.

```rust
use tekes_ui_controls::{composition::IconContent, extended::StatusBadge};
use tekes_ui_tokens::surface_palette::resolve_surface;

let packet = resolve_surface(palette, declared_surface,
    "content-status-running", "rest")?;
let mut icon = IconContent::new("tekes-settings")?
    .with_accessible_name("Running");
icon.set_status_tone(Some(StatusBadge::Running));
icon.set_disabled(false);
// Mount in the original Content tree; inherit the container's declared Surface.
```

`None` selects ordinary content. The named passive node has explicit text semantics and disabled state; it introduces no action or focus target. Decorative action children consume the original `ContentTheme.icon_foreground` first, including original disabled/selected/hover/pressed results. Passive status tone or the child Icon's disabled flag must never overwrite on-accent action ink. Non-Primary original Button variants keep action-quiet; Panel is not a new action-panel use. Original loading blocks activation and retains its existing rest paint.

Use the existing checked IconRenderer service and `WithIconRenderer`; the public `svg-icons` service reuses canonical assets and original GPU image ownership. Missing service, failed resource preparation or an unavailable status packet is an explicit error. There is no character, arbitrary-color or partial-packet fallback. Original Icon sizing, motion context, reduced-motion and frame-permission contracts remain component-owned.

## Validation, extension and evidence

Every new Surface must declare all31 uses and the exact states/channels each use supports. An old27-use extension must migrate explicitly to Schema1.2. Unknown Surface/use/profile/role, missing status use/icon, invented passive hover/focus, own status ground and RGB identity fields are rejected. Python generation and ordinary Rust compilation enforce the same rules and exact generated input hashes. Invalid normal CLI generation leaves existing outputs unchanged. Generated output is deterministic across processes; no hand-edited generated table is accepted.

The underlying rules1.6 status-only implementation originated in candidate manifest `37ffd84933ec4fa4a4a74414fe4c92697f01ea6dc0f174af18d6e1f21c6645e2`. It excludes the earlier Progress-only rules1.6 proposal. Promotion updates this current contract and its DS reference without changing the candidate's eleven other core files. Original candidate and component1441 inputs remain frozen.

Source-bound acceptance: [5232 original component/SVG GPU frames](icon-status-complete-GPU5232-review.json), [432 public declared-page GPU frames](icon-public-declared-page-GPU432-review.json), [144 canonical Gallery before/after pairs](icon-whole-page144-before-after-pixel-equivalence.json), and [actual Gallery caller/Native-vector repair](icon-palette-final-caller-GPU80-review.json). The real public example's action processing preserves synthetic Composer marked text, selection and focus binding; its384 after-frame Composer regions match the before pixels exactly. Actual native display, physical IME, VoiceOver, complete Gold7, remaining component families and final registry acceptance remain separately tracked in #55/#56. This is local WIP, not a published package revision.

## Public Link consumption

```rust,ignore
use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, LinkContent};
let id = ComponentId(1);
let mut host = ContentHost::new(Content::new(id, LinkContent::new("Open documentation")))?;
host.set_inherited_surface("panel")?;
host.set_action_events(true);
host.get_mut(id).unwrap().set_surface(Some("code"))?;
host.get_mut(id).unwrap().set_surface(None)?; // Resume inherited panel.
let actions = host.drain_action_records(); // Product handles each original command once.
```

No consumer state-color table is required. The original Link model selects rest/hover/pressed/disabled; focus-visible paints an independent focus stroke with the generated focus ink. Background and optional boundary belong to the same packet. Shared measured text bounds constrain rounding so multiline inverse text keeps its ground; host frame, hit region and accessibility frame retain their original ownership. Geometry corrections do not change Theme colors or token versions. Use the original declared Composer+Link example for retained-input integration rather than replacing an existing product tree with a synthetic theme demo.
