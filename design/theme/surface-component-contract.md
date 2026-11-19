# Theme/Surface API rules 1.9.0

Additive to rules 1.8.0, schema 1.3.0: the original 37 usages, states, seven-channel packets and compiled color role maps remain unchanged. Palette, resolve_surface and SurfaceColors APIs are unchanged.

Five rest-only usages: notice-marker-info→status-info; notice-marker-success→status-succeeded; notice-marker-error→status-failed; notice-marker-toast-warning→status-failed; notice-marker-banner-warning→status-running. These are marker roles, not business NoticeTone names. Warning remains Warning.

All five require background=$surface, text/icon=primary-text, exact marker above, boundary/focus/helper=null. All seven fields are mandatory. Every other state rejects. Marker ink contrasts with the actual Surface ground at >=3, text/icon follow the existing >=4.5 rule. No new role, token, base color, required boundary or badge on-fill borrowing.

The seven required Surfaces and explicit extra Surface support remain. Default and all existing Theme definitions compile all four modes. Independent consumer artifacts cannot substitute shared rules. Python offline generation and normal Rust validation enforce bindings, states and actual-parent contrast; synchronized schema const, version policy, input pins, output and independent source locks bind the result.

Component View/Content, original Gallery and GPU consumption belong to #79. This contract alone does not validate those paths.
