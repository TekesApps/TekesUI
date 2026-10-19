# Historical Gallery reference environment repair (#2)

The macOS CI failure at `primitives-light@2x (652,145)` reproduces locally with process-only `-AppleLanguages (en-US)`: the expected pixel `[252,252,251,255]` becomes `[242,242,241,255]`. Both hosts run macOS 26.6.2 (25G83). Controlled CoreText probes resolve 圆角 through PingFang under English and Hiragino under the recovered reference language profile. The profile is recovered evidence, not a claim that the original capture recorded its environment.

The historical golden test now launches only its own exact test in a subprocess with the recovered language profile. NSUserDefaults reads the process argument domain; no system preference is modified. Product font fallback, the original 24 PNGs, comparison tolerance and original verification metadata are unchanged. Parallel tests keep their ambient language configuration.

Local verification on the independent branch based on 431093786de47b48cd566d0d9c0eafa088b10d68: workspace tests 923 passed, 1 existing ignored; independent public consumer 131 passed; format, strict workspace/all-target Clippy and locked check passed. The original English configuration failed before this change and passed after it. The 188 reviewed AgentKit public inputs still match 03721866a362b1d31261ebbbc86140eb07e9e23320871c325234a5212591213d. Remote CI is separate evidence and must complete on the published repair SHA.

Applicability: historical macOS screenshot comparison only. This is not whole-Gallery native/VoiceOver or #1–#14 acceptance. AgentKit's accepted pin and branch remain unchanged.

Follow-up: remote run 39222725239 passed all Gallery tests including the isolated golden, then exposed another language-specific assumption in `non_system_font_cjk_fallback_makes_the_line_taller`: English fallback has zero leading. The test now compares Latin/CJK/mixed ascent, descent, leading and height directly with independent CTLine typographic bounds, and still checks fallback identity and per-line/multiline behavior. No production text code changed. Local 15 shape tests and strict package/all-target Clippy passed; final remote rerun remains required.
