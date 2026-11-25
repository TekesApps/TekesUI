//! Optional product-specific icons gated behind the `product-icons` Cargo feature.
use crate::Glyph;

pub(crate) const EXTENSIONS: &[Glyph] = &[
    Glyph {
        name: "tekes-agent-question",
        svg: include_str!("../packs/product-extensions/assets/tekes-agent-question.svg"),
    },
    Glyph {
        name: "tekes-ask-user",
        svg: include_str!("../packs/product-extensions/assets/tekes-ask-user.svg"),
    },
    Glyph {
        name: "tekes-cad-arc",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-arc.svg"),
    },
    Glyph {
        name: "tekes-cad-block",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-block.svg"),
    },
    Glyph {
        name: "tekes-cad-compare",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-compare.svg"),
    },
    Glyph {
        name: "tekes-cad-crossing-select",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-crossing-select.svg"),
    },
    Glyph {
        name: "tekes-cad-dimension",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-dimension.svg"),
    },
    Glyph {
        name: "tekes-cad-export",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-export.svg"),
    },
    Glyph {
        name: "tekes-cad-layers",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-layers.svg"),
    },
    Glyph {
        name: "tekes-cad-layout",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-layout.svg"),
    },
    Glyph {
        name: "tekes-cad-line",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-line.svg"),
    },
    Glyph {
        name: "tekes-cad-pdf-import",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-pdf-import.svg"),
    },
    Glyph {
        name: "tekes-cad-polyline",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-polyline.svg"),
    },
    Glyph {
        name: "tekes-cad-snap",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-snap.svg"),
    },
    Glyph {
        name: "tekes-cad-table-extract",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-table-extract.svg"),
    },
    Glyph {
        name: "tekes-cad-window-select",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-window-select.svg"),
    },
    Glyph {
        name: "tekes-drama-audience",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-audience.svg"),
    },
    Glyph {
        name: "tekes-drama-audio",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-audio.svg"),
    },
    Glyph {
        name: "tekes-drama-brief",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-brief.svg"),
    },
    Glyph {
        name: "tekes-drama-character",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-character.svg"),
    },
    Glyph {
        name: "tekes-drama-collaboration",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-collaboration.svg"),
    },
    Glyph {
        name: "tekes-drama-delivery",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-delivery.svg"),
    },
    Glyph {
        name: "tekes-drama-directing",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-directing.svg"),
    },
    Glyph {
        name: "tekes-drama-editing",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-editing.svg"),
    },
    Glyph {
        name: "tekes-drama-production",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-production.svg"),
    },
    Glyph {
        name: "tekes-drama-references",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-references.svg"),
    },
    Glyph {
        name: "tekes-drama-script",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-script.svg"),
    },
    Glyph {
        name: "tekes-drama-shot",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-shot.svg"),
    },
    Glyph {
        name: "tekes-drama-storyboard",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-storyboard.svg"),
    },
    Glyph {
        name: "tekes-drama-subtitles",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-subtitles.svg"),
    },
    Glyph {
        name: "tekes-goal",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal.svg"),
    },
    Glyph {
        name: "tekes-goal-state-active",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-active.svg"),
    },
    Glyph {
        name: "tekes-goal-state-blocked",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-blocked.svg"),
    },
    Glyph {
        name: "tekes-goal-state-completed",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-completed.svg"),
    },
    Glyph {
        name: "tekes-goal-state-stopped",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-stopped.svg"),
    },
    Glyph {
        name: "tekes-mcp",
        svg: include_str!("../packs/product-extensions/assets/tekes-mcp.svg"),
    },
    Glyph {
        name: "tekes-memorize",
        svg: include_str!("../packs/product-extensions/assets/tekes-memorize.svg"),
    },
    Glyph {
        name: "tekes-queued-input",
        svg: include_str!("../packs/product-extensions/assets/tekes-queued-input.svg"),
    },
    Glyph {
        name: "tekes-recall",
        svg: include_str!("../packs/product-extensions/assets/tekes-recall.svg"),
    },
    Glyph {
        name: "tekes-skill",
        svg: include_str!("../packs/product-extensions/assets/tekes-skill.svg"),
    },
    Glyph {
        name: "tekes-skills",
        svg: include_str!("../packs/product-extensions/assets/tekes-skills.svg"),
    },
    Glyph {
        name: "tekes-slash-command",
        svg: include_str!("../packs/product-extensions/assets/tekes-slash-command.svg"),
    },
    Glyph {
        name: "tekes-steer",
        svg: include_str!("../packs/product-extensions/assets/tekes-steer.svg"),
    },
    Glyph {
        name: "tekes-steering",
        svg: include_str!("../packs/product-extensions/assets/tekes-steering.svg"),
    },
    Glyph {
        name: "tekes-subagent",
        svg: include_str!("../packs/product-extensions/assets/tekes-subagent.svg"),
    },
    Glyph {
        name: "tekes-waiting-user",
        svg: include_str!("../packs/product-extensions/assets/tekes-waiting-user.svg"),
    },
];

pub(crate) const ALL: &[Glyph] = &[
    Glyph {
        name: "tekes-access-full",
        svg: include_str!("../assets/tekes-access-full.svg"),
    },
    Glyph {
        name: "tekes-access-read",
        svg: include_str!("../assets/tekes-access-read.svg"),
    },
    Glyph {
        name: "tekes-advanced-params",
        svg: include_str!("../assets/tekes-advanced-params.svg"),
    },
    Glyph {
        name: "tekes-agent-question",
        svg: include_str!("../packs/product-extensions/assets/tekes-agent-question.svg"),
    },
    Glyph {
        name: "tekes-ahead-behind",
        svg: include_str!("../assets/tekes-ahead-behind.svg"),
    },
    Glyph {
        name: "tekes-annotate",
        svg: include_str!("../assets/tekes-annotate.svg"),
    },
    Glyph {
        name: "tekes-app-server",
        svg: include_str!("../assets/tekes-app-server.svg"),
    },
    Glyph {
        name: "tekes-appearance",
        svg: include_str!("../assets/tekes-appearance.svg"),
    },
    Glyph {
        name: "tekes-approval",
        svg: include_str!("../assets/tekes-approval.svg"),
    },
    Glyph {
        name: "tekes-archive",
        svg: include_str!("../assets/tekes-archive.svg"),
    },
    Glyph {
        name: "tekes-ask-user",
        svg: include_str!("../packs/product-extensions/assets/tekes-ask-user.svg"),
    },
    Glyph {
        name: "tekes-attach",
        svg: include_str!("../assets/tekes-attach.svg"),
    },
    Glyph {
        name: "tekes-authorized-fill",
        svg: include_str!("../assets/tekes-authorized-fill.svg"),
    },
    Glyph {
        name: "tekes-back",
        svg: include_str!("../assets/tekes-back.svg"),
    },
    Glyph {
        name: "tekes-branch",
        svg: include_str!("../assets/tekes-branch.svg"),
    },
    Glyph {
        name: "tekes-brief",
        svg: include_str!("../assets/tekes-brief.svg"),
    },
    Glyph {
        name: "tekes-cad-arc",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-arc.svg"),
    },
    Glyph {
        name: "tekes-cad-block",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-block.svg"),
    },
    Glyph {
        name: "tekes-cad-compare",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-compare.svg"),
    },
    Glyph {
        name: "tekes-cad-crossing-select",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-crossing-select.svg"),
    },
    Glyph {
        name: "tekes-cad-dimension",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-dimension.svg"),
    },
    Glyph {
        name: "tekes-cad-export",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-export.svg"),
    },
    Glyph {
        name: "tekes-cad-layers",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-layers.svg"),
    },
    Glyph {
        name: "tekes-cad-layout",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-layout.svg"),
    },
    Glyph {
        name: "tekes-cad-line",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-line.svg"),
    },
    Glyph {
        name: "tekes-cad-pdf-import",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-pdf-import.svg"),
    },
    Glyph {
        name: "tekes-cad-polyline",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-polyline.svg"),
    },
    Glyph {
        name: "tekes-cad-snap",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-snap.svg"),
    },
    Glyph {
        name: "tekes-cad-table-extract",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-table-extract.svg"),
    },
    Glyph {
        name: "tekes-cad-window-select",
        svg: include_str!("../packs/product-extensions/assets/tekes-cad-window-select.svg"),
    },
    Glyph {
        name: "tekes-chat",
        svg: include_str!("../assets/tekes-chat.svg"),
    },
    Glyph {
        name: "tekes-check-circle",
        svg: include_str!("../assets/tekes-check-circle.svg"),
    },
    Glyph {
        name: "tekes-clear-group",
        svg: include_str!("../assets/tekes-clear-group.svg"),
    },
    Glyph {
        name: "tekes-collapse",
        svg: include_str!("../assets/tekes-collapse.svg"),
    },
    Glyph {
        name: "tekes-command",
        svg: include_str!("../assets/tekes-command.svg"),
    },
    Glyph {
        name: "tekes-commands",
        svg: include_str!("../assets/tekes-commands.svg"),
    },
    Glyph {
        name: "tekes-commit",
        svg: include_str!("../assets/tekes-commit.svg"),
    },
    Glyph {
        name: "tekes-compact",
        svg: include_str!("../assets/tekes-compact.svg"),
    },
    Glyph {
        name: "tekes-compare",
        svg: include_str!("../assets/tekes-compare.svg"),
    },
    Glyph {
        name: "tekes-confirm",
        svg: include_str!("../assets/tekes-confirm.svg"),
    },
    Glyph {
        name: "tekes-context",
        svg: include_str!("../assets/tekes-context.svg"),
    },
    Glyph {
        name: "tekes-context-usage",
        svg: include_str!("../assets/tekes-context-usage.svg"),
    },
    Glyph {
        name: "tekes-copy",
        svg: include_str!("../assets/tekes-copy.svg"),
    },
    Glyph {
        name: "tekes-delete",
        svg: include_str!("../assets/tekes-delete.svg"),
    },
    Glyph {
        name: "tekes-desktop-control",
        svg: include_str!("../assets/tekes-desktop-control.svg"),
    },
    Glyph {
        name: "tekes-diff-annotation",
        svg: include_str!("../assets/tekes-diff-annotation.svg"),
    },
    Glyph {
        name: "tekes-diff-list",
        svg: include_str!("../assets/tekes-diff-list.svg"),
    },
    Glyph {
        name: "tekes-diff-split-h",
        svg: include_str!("../assets/tekes-diff-split-h.svg"),
    },
    Glyph {
        name: "tekes-diff-split-v",
        svg: include_str!("../assets/tekes-diff-split-v.svg"),
    },
    Glyph {
        name: "tekes-drama-audience",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-audience.svg"),
    },
    Glyph {
        name: "tekes-drama-audio",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-audio.svg"),
    },
    Glyph {
        name: "tekes-drama-brief",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-brief.svg"),
    },
    Glyph {
        name: "tekes-drama-character",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-character.svg"),
    },
    Glyph {
        name: "tekes-drama-collaboration",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-collaboration.svg"),
    },
    Glyph {
        name: "tekes-drama-delivery",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-delivery.svg"),
    },
    Glyph {
        name: "tekes-drama-directing",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-directing.svg"),
    },
    Glyph {
        name: "tekes-drama-editing",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-editing.svg"),
    },
    Glyph {
        name: "tekes-drama-production",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-production.svg"),
    },
    Glyph {
        name: "tekes-drama-references",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-references.svg"),
    },
    Glyph {
        name: "tekes-drama-script",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-script.svg"),
    },
    Glyph {
        name: "tekes-drama-shot",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-shot.svg"),
    },
    Glyph {
        name: "tekes-drama-storyboard",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-storyboard.svg"),
    },
    Glyph {
        name: "tekes-drama-subtitles",
        svg: include_str!("../packs/product-extensions/assets/tekes-drama-subtitles.svg"),
    },
    Glyph {
        name: "tekes-edit",
        svg: include_str!("../assets/tekes-edit.svg"),
    },
    Glyph {
        name: "tekes-error",
        svg: include_str!("../assets/tekes-error.svg"),
    },
    Glyph {
        name: "tekes-error-fill",
        svg: include_str!("../assets/tekes-error-fill.svg"),
    },
    Glyph {
        name: "tekes-expand",
        svg: include_str!("../assets/tekes-expand.svg"),
    },
    Glyph {
        name: "tekes-file-added",
        svg: include_str!("../assets/tekes-file-added.svg"),
    },
    Glyph {
        name: "tekes-file-code",
        svg: include_str!("../assets/tekes-file-code.svg"),
    },
    Glyph {
        name: "tekes-file-generic",
        svg: include_str!("../assets/tekes-file-generic.svg"),
    },
    Glyph {
        name: "tekes-file-link",
        svg: include_str!("../assets/tekes-file-link.svg"),
    },
    Glyph {
        name: "tekes-file-rich",
        svg: include_str!("../assets/tekes-file-rich.svg"),
    },
    Glyph {
        name: "tekes-file-text",
        svg: include_str!("../assets/tekes-file-text.svg"),
    },
    Glyph {
        name: "tekes-files",
        svg: include_str!("../assets/tekes-files.svg"),
    },
    Glyph {
        name: "tekes-filter",
        svg: include_str!("../assets/tekes-filter.svg"),
    },
    Glyph {
        name: "tekes-flash",
        svg: include_str!("../assets/tekes-flash.svg"),
    },
    Glyph {
        name: "tekes-flash-fill",
        svg: include_str!("../assets/tekes-flash-fill.svg"),
    },
    Glyph {
        name: "tekes-folder",
        svg: include_str!("../assets/tekes-folder.svg"),
    },
    Glyph {
        name: "tekes-folder-edit",
        svg: include_str!("../assets/tekes-folder-edit.svg"),
    },
    Glyph {
        name: "tekes-folder-open",
        svg: include_str!("../assets/tekes-folder-open.svg"),
    },
    Glyph {
        name: "tekes-fork",
        svg: include_str!("../assets/tekes-fork.svg"),
    },
    Glyph {
        name: "tekes-forward",
        svg: include_str!("../assets/tekes-forward.svg"),
    },
    Glyph {
        name: "tekes-fullscreen",
        svg: include_str!("../assets/tekes-fullscreen.svg"),
    },
    Glyph {
        name: "tekes-goal",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal.svg"),
    },
    Glyph {
        name: "tekes-goal-state-active",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-active.svg"),
    },
    Glyph {
        name: "tekes-goal-state-blocked",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-blocked.svg"),
    },
    Glyph {
        name: "tekes-goal-state-completed",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-completed.svg"),
    },
    Glyph {
        name: "tekes-goal-state-stopped",
        svg: include_str!("../packs/product-extensions/assets/tekes-goal-state-stopped.svg"),
    },
    Glyph {
        name: "tekes-hide",
        svg: include_str!("../assets/tekes-hide.svg"),
    },
    Glyph {
        name: "tekes-image",
        svg: include_str!("../assets/tekes-image.svg"),
    },
    Glyph {
        name: "tekes-info",
        svg: include_str!("../assets/tekes-info.svg"),
    },
    Glyph {
        name: "tekes-job",
        svg: include_str!("../assets/tekes-job.svg"),
    },
    Glyph {
        name: "tekes-jump-bottom",
        svg: include_str!("../assets/tekes-jump-bottom.svg"),
    },
    Glyph {
        name: "tekes-lock-fill",
        svg: include_str!("../assets/tekes-lock-fill.svg"),
    },
    Glyph {
        name: "tekes-logout",
        svg: include_str!("../assets/tekes-logout.svg"),
    },
    Glyph {
        name: "tekes-mcp",
        svg: include_str!("../packs/product-extensions/assets/tekes-mcp.svg"),
    },
    Glyph {
        name: "tekes-memorize",
        svg: include_str!("../packs/product-extensions/assets/tekes-memorize.svg"),
    },
    Glyph {
        name: "tekes-memory",
        svg: include_str!("../assets/tekes-memory.svg"),
    },
    Glyph {
        name: "tekes-mention",
        svg: include_str!("../assets/tekes-mention.svg"),
    },
    Glyph {
        name: "tekes-merge",
        svg: include_str!("../assets/tekes-merge.svg"),
    },
    Glyph {
        name: "tekes-more",
        svg: include_str!("../assets/tekes-more.svg"),
    },
    Glyph {
        name: "tekes-new",
        svg: include_str!("../assets/tekes-new.svg"),
    },
    Glyph {
        name: "tekes-new-goal",
        svg: include_str!("../assets/tekes-new-goal.svg"),
    },
    Glyph {
        name: "tekes-new-project",
        svg: include_str!("../assets/tekes-new-project.svg"),
    },
    Glyph {
        name: "tekes-no-default-app",
        svg: include_str!("../assets/tekes-no-default-app.svg"),
    },
    Glyph {
        name: "tekes-note",
        svg: include_str!("../assets/tekes-note.svg"),
    },
    Glyph {
        name: "tekes-oauth-key",
        svg: include_str!("../assets/tekes-oauth-key.svg"),
    },
    Glyph {
        name: "tekes-open-external",
        svg: include_str!("../assets/tekes-open-external.svg"),
    },
    Glyph {
        name: "tekes-pause",
        svg: include_str!("../assets/tekes-pause.svg"),
    },
    Glyph {
        name: "tekes-pending-task",
        svg: include_str!("../assets/tekes-pending-task.svg"),
    },
    Glyph {
        name: "tekes-pin",
        svg: include_str!("../assets/tekes-pin.svg"),
    },
    Glyph {
        name: "tekes-pinned",
        svg: include_str!("../assets/tekes-pinned.svg"),
    },
    Glyph {
        name: "tekes-plugin",
        svg: include_str!("../assets/tekes-plugin.svg"),
    },
    Glyph {
        name: "tekes-plugins",
        svg: include_str!("../assets/tekes-plugins.svg"),
    },
    Glyph {
        name: "tekes-popup-menu",
        svg: include_str!("../assets/tekes-popup-menu.svg"),
    },
    Glyph {
        name: "tekes-profile",
        svg: include_str!("../assets/tekes-profile.svg"),
    },
    Glyph {
        name: "tekes-progress",
        svg: include_str!("../assets/tekes-progress.svg"),
    },
    Glyph {
        name: "tekes-provider",
        svg: include_str!("../assets/tekes-provider.svg"),
    },
    Glyph {
        name: "tekes-pull-request",
        svg: include_str!("../assets/tekes-pull-request.svg"),
    },
    Glyph {
        name: "tekes-queued-input",
        svg: include_str!("../packs/product-extensions/assets/tekes-queued-input.svg"),
    },
    Glyph {
        name: "tekes-quick-panel",
        svg: include_str!("../assets/tekes-quick-panel.svg"),
    },
    Glyph {
        name: "tekes-quote",
        svg: include_str!("../assets/tekes-quote.svg"),
    },
    Glyph {
        name: "tekes-read",
        svg: include_str!("../assets/tekes-read.svg"),
    },
    Glyph {
        name: "tekes-rebuild",
        svg: include_str!("../assets/tekes-rebuild.svg"),
    },
    Glyph {
        name: "tekes-recall",
        svg: include_str!("../packs/product-extensions/assets/tekes-recall.svg"),
    },
    Glyph {
        name: "tekes-redo",
        svg: include_str!("../assets/tekes-redo.svg"),
    },
    Glyph {
        name: "tekes-refresh",
        svg: include_str!("../assets/tekes-refresh.svg"),
    },
    Glyph {
        name: "tekes-remove-one",
        svg: include_str!("../assets/tekes-remove-one.svg"),
    },
    Glyph {
        name: "tekes-report",
        svg: include_str!("../assets/tekes-report.svg"),
    },
    Glyph {
        name: "tekes-resume",
        svg: include_str!("../assets/tekes-resume.svg"),
    },
    Glyph {
        name: "tekes-review-plan",
        svg: include_str!("../assets/tekes-review-plan.svg"),
    },
    Glyph {
        name: "tekes-rewind",
        svg: include_str!("../assets/tekes-rewind.svg"),
    },
    Glyph {
        name: "tekes-router",
        svg: include_str!("../assets/tekes-router.svg"),
    },
    Glyph {
        name: "tekes-scheduled",
        svg: include_str!("../assets/tekes-scheduled.svg"),
    },
    Glyph {
        name: "tekes-search",
        svg: include_str!("../assets/tekes-search.svg"),
    },
    Glyph {
        name: "tekes-send",
        svg: include_str!("../assets/tekes-send.svg"),
    },
    Glyph {
        name: "tekes-settings",
        svg: include_str!("../assets/tekes-settings.svg"),
    },
    Glyph {
        name: "tekes-shell",
        svg: include_str!("../assets/tekes-shell.svg"),
    },
    Glyph {
        name: "tekes-show",
        svg: include_str!("../assets/tekes-show.svg"),
    },
    Glyph {
        name: "tekes-side-bar",
        svg: include_str!("../assets/tekes-side-bar.svg"),
    },
    Glyph {
        name: "tekes-side-bar-collapsed",
        svg: include_str!("../assets/tekes-side-bar-collapsed.svg"),
    },
    Glyph {
        name: "tekes-side-panel",
        svg: include_str!("../assets/tekes-side-panel.svg"),
    },
    Glyph {
        name: "tekes-side-panel-collapsed",
        svg: include_str!("../assets/tekes-side-panel-collapsed.svg"),
    },
    Glyph {
        name: "tekes-skill",
        svg: include_str!("../packs/product-extensions/assets/tekes-skill.svg"),
    },
    Glyph {
        name: "tekes-skills",
        svg: include_str!("../packs/product-extensions/assets/tekes-skills.svg"),
    },
    Glyph {
        name: "tekes-slash-command",
        svg: include_str!("../packs/product-extensions/assets/tekes-slash-command.svg"),
    },
    Glyph {
        name: "tekes-sort",
        svg: include_str!("../assets/tekes-sort.svg"),
    },
    Glyph {
        name: "tekes-steer",
        svg: include_str!("../packs/product-extensions/assets/tekes-steer.svg"),
    },
    Glyph {
        name: "tekes-steering",
        svg: include_str!("../packs/product-extensions/assets/tekes-steering.svg"),
    },
    Glyph {
        name: "tekes-stop",
        svg: include_str!("../assets/tekes-stop.svg"),
    },
    Glyph {
        name: "tekes-subagent",
        svg: include_str!("../packs/product-extensions/assets/tekes-subagent.svg"),
    },
    Glyph {
        name: "tekes-swap-horizontal",
        svg: include_str!("../assets/tekes-swap-horizontal.svg"),
    },
    Glyph {
        name: "tekes-tab-diffs",
        svg: include_str!("../assets/tekes-tab-diffs.svg"),
    },
    Glyph {
        name: "tekes-tab-summary",
        svg: include_str!("../assets/tekes-tab-summary.svg"),
    },
    Glyph {
        name: "tekes-tag",
        svg: include_str!("../assets/tekes-tag.svg"),
    },
    Glyph {
        name: "tekes-task",
        svg: include_str!("../assets/tekes-task.svg"),
    },
    Glyph {
        name: "tekes-test-connection",
        svg: include_str!("../assets/tekes-test-connection.svg"),
    },
    Glyph {
        name: "tekes-think",
        svg: include_str!("../assets/tekes-think.svg"),
    },
    Glyph {
        name: "tekes-thumbs-down",
        svg: include_str!("../assets/tekes-thumbs-down.svg"),
    },
    Glyph {
        name: "tekes-thumbs-down-fill",
        svg: include_str!("../assets/tekes-thumbs-down-fill.svg"),
    },
    Glyph {
        name: "tekes-thumbs-up",
        svg: include_str!("../assets/tekes-thumbs-up.svg"),
    },
    Glyph {
        name: "tekes-thumbs-up-fill",
        svg: include_str!("../assets/tekes-thumbs-up-fill.svg"),
    },
    Glyph {
        name: "tekes-time",
        svg: include_str!("../assets/tekes-time.svg"),
    },
    Glyph {
        name: "tekes-tools",
        svg: include_str!("../assets/tekes-tools.svg"),
    },
    Glyph {
        name: "tekes-trust",
        svg: include_str!("../assets/tekes-trust.svg"),
    },
    Glyph {
        name: "tekes-undo",
        svg: include_str!("../assets/tekes-undo.svg"),
    },
    Glyph {
        name: "tekes-unknown-fallback",
        svg: include_str!("../assets/tekes-unknown-fallback.svg"),
    },
    Glyph {
        name: "tekes-unknown-tool",
        svg: include_str!("../assets/tekes-unknown-tool.svg"),
    },
    Glyph {
        name: "tekes-unpushed",
        svg: include_str!("../assets/tekes-unpushed.svg"),
    },
    Glyph {
        name: "tekes-untracked-folder",
        svg: include_str!("../assets/tekes-untracked-folder.svg"),
    },
    Glyph {
        name: "tekes-verify",
        svg: include_str!("../assets/tekes-verify.svg"),
    },
    Glyph {
        name: "tekes-waiting-user",
        svg: include_str!("../packs/product-extensions/assets/tekes-waiting-user.svg"),
    },
    Glyph {
        name: "tekes-warning",
        svg: include_str!("../assets/tekes-warning.svg"),
    },
    Glyph {
        name: "tekes-warning-fill",
        svg: include_str!("../assets/tekes-warning-fill.svg"),
    },
    Glyph {
        name: "tekes-web-fetch",
        svg: include_str!("../assets/tekes-web-fetch.svg"),
    },
    Glyph {
        name: "tekes-web-search",
        svg: include_str!("../assets/tekes-web-search.svg"),
    },
    Glyph {
        name: "tekes-write",
        svg: include_str!("../assets/tekes-write.svg"),
    },
    Glyph {
        name: "tekes-zoom-in",
        svg: include_str!("../assets/tekes-zoom-in.svg"),
    },
    Glyph {
        name: "tekes-zoom-out",
        svg: include_str!("../assets/tekes-zoom-out.svg"),
    },
];
