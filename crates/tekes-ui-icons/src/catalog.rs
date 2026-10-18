//! Complete published catalog. `Icon` remains the eight-icon animated compatibility API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub name: &'static str,
    pub svg: &'static str,
}
impl Glyph {
    pub const ALL: &'static [Self] = &[
        Self {
            name: "tekes-access-full",
            svg: include_str!("../assets/tekes-access-full.svg"),
        },
        Self {
            name: "tekes-access-read",
            svg: include_str!("../assets/tekes-access-read.svg"),
        },
        Self {
            name: "tekes-access-workspace",
            svg: include_str!("../assets/tekes-access-workspace.svg"),
        },
        Self {
            name: "tekes-advanced-params",
            svg: include_str!("../assets/tekes-advanced-params.svg"),
        },
        Self {
            name: "tekes-agent-question",
            svg: include_str!("../assets/tekes-agent-question.svg"),
        },
        Self {
            name: "tekes-ahead-behind",
            svg: include_str!("../assets/tekes-ahead-behind.svg"),
        },
        Self {
            name: "tekes-annotate",
            svg: include_str!("../assets/tekes-annotate.svg"),
        },
        Self {
            name: "tekes-app-server",
            svg: include_str!("../assets/tekes-app-server.svg"),
        },
        Self {
            name: "tekes-appearance",
            svg: include_str!("../assets/tekes-appearance.svg"),
        },
        Self {
            name: "tekes-approval",
            svg: include_str!("../assets/tekes-approval.svg"),
        },
        Self {
            name: "tekes-archive",
            svg: include_str!("../assets/tekes-archive.svg"),
        },
        Self {
            name: "tekes-ask-user",
            svg: include_str!("../assets/tekes-ask-user.svg"),
        },
        Self {
            name: "tekes-attach",
            svg: include_str!("../assets/tekes-attach.svg"),
        },
        Self {
            name: "tekes-authorized-fill",
            svg: include_str!("../assets/tekes-authorized-fill.svg"),
        },
        Self {
            name: "tekes-back",
            svg: include_str!("../assets/tekes-back.svg"),
        },
        Self {
            name: "tekes-branch",
            svg: include_str!("../assets/tekes-branch.svg"),
        },
        Self {
            name: "tekes-brief",
            svg: include_str!("../assets/tekes-brief.svg"),
        },
        Self {
            name: "tekes-cad-arc",
            svg: include_str!("../assets/tekes-cad-arc.svg"),
        },
        Self {
            name: "tekes-cad-block",
            svg: include_str!("../assets/tekes-cad-block.svg"),
        },
        Self {
            name: "tekes-cad-compare",
            svg: include_str!("../assets/tekes-cad-compare.svg"),
        },
        Self {
            name: "tekes-cad-crossing-select",
            svg: include_str!("../assets/tekes-cad-crossing-select.svg"),
        },
        Self {
            name: "tekes-cad-dimension",
            svg: include_str!("../assets/tekes-cad-dimension.svg"),
        },
        Self {
            name: "tekes-cad-export",
            svg: include_str!("../assets/tekes-cad-export.svg"),
        },
        Self {
            name: "tekes-cad-layers",
            svg: include_str!("../assets/tekes-cad-layers.svg"),
        },
        Self {
            name: "tekes-cad-layout",
            svg: include_str!("../assets/tekes-cad-layout.svg"),
        },
        Self {
            name: "tekes-cad-line",
            svg: include_str!("../assets/tekes-cad-line.svg"),
        },
        Self {
            name: "tekes-cad-pdf-import",
            svg: include_str!("../assets/tekes-cad-pdf-import.svg"),
        },
        Self {
            name: "tekes-cad-polyline",
            svg: include_str!("../assets/tekes-cad-polyline.svg"),
        },
        Self {
            name: "tekes-cad-snap",
            svg: include_str!("../assets/tekes-cad-snap.svg"),
        },
        Self {
            name: "tekes-cad-table-extract",
            svg: include_str!("../assets/tekes-cad-table-extract.svg"),
        },
        Self {
            name: "tekes-cad-window-select",
            svg: include_str!("../assets/tekes-cad-window-select.svg"),
        },
        Self {
            name: "tekes-clear-group",
            svg: include_str!("../assets/tekes-clear-group.svg"),
        },
        Self {
            name: "tekes-collapse",
            svg: include_str!("../assets/tekes-collapse.svg"),
        },
        Self {
            name: "tekes-command",
            svg: include_str!("../assets/tekes-command.svg"),
        },
        Self {
            name: "tekes-commands",
            svg: include_str!("../assets/tekes-commands.svg"),
        },
        Self {
            name: "tekes-commit",
            svg: include_str!("../assets/tekes-commit.svg"),
        },
        Self {
            name: "tekes-compact",
            svg: include_str!("../assets/tekes-compact.svg"),
        },
        Self {
            name: "tekes-compare",
            svg: include_str!("../assets/tekes-compare.svg"),
        },
        Self {
            name: "tekes-confirm",
            svg: include_str!("../assets/tekes-confirm.svg"),
        },
        Self {
            name: "tekes-context",
            svg: include_str!("../assets/tekes-context.svg"),
        },
        Self {
            name: "tekes-context-usage",
            svg: include_str!("../assets/tekes-context-usage.svg"),
        },
        Self {
            name: "tekes-copy",
            svg: include_str!("../assets/tekes-copy.svg"),
        },
        Self {
            name: "tekes-delete",
            svg: include_str!("../assets/tekes-delete.svg"),
        },
        Self {
            name: "tekes-desktop-control",
            svg: include_str!("../assets/tekes-desktop-control.svg"),
        },
        Self {
            name: "tekes-diff-annotation",
            svg: include_str!("../assets/tekes-diff-annotation.svg"),
        },
        Self {
            name: "tekes-diff-list",
            svg: include_str!("../assets/tekes-diff-list.svg"),
        },
        Self {
            name: "tekes-diff-split-h",
            svg: include_str!("../assets/tekes-diff-split-h.svg"),
        },
        Self {
            name: "tekes-diff-split-v",
            svg: include_str!("../assets/tekes-diff-split-v.svg"),
        },
        Self {
            name: "tekes-drama-audience",
            svg: include_str!("../assets/tekes-drama-audience.svg"),
        },
        Self {
            name: "tekes-drama-audio",
            svg: include_str!("../assets/tekes-drama-audio.svg"),
        },
        Self {
            name: "tekes-drama-brief",
            svg: include_str!("../assets/tekes-drama-brief.svg"),
        },
        Self {
            name: "tekes-drama-character",
            svg: include_str!("../assets/tekes-drama-character.svg"),
        },
        Self {
            name: "tekes-drama-collaboration",
            svg: include_str!("../assets/tekes-drama-collaboration.svg"),
        },
        Self {
            name: "tekes-drama-delivery",
            svg: include_str!("../assets/tekes-drama-delivery.svg"),
        },
        Self {
            name: "tekes-drama-directing",
            svg: include_str!("../assets/tekes-drama-directing.svg"),
        },
        Self {
            name: "tekes-drama-editing",
            svg: include_str!("../assets/tekes-drama-editing.svg"),
        },
        Self {
            name: "tekes-drama-production",
            svg: include_str!("../assets/tekes-drama-production.svg"),
        },
        Self {
            name: "tekes-drama-references",
            svg: include_str!("../assets/tekes-drama-references.svg"),
        },
        Self {
            name: "tekes-drama-script",
            svg: include_str!("../assets/tekes-drama-script.svg"),
        },
        Self {
            name: "tekes-drama-shot",
            svg: include_str!("../assets/tekes-drama-shot.svg"),
        },
        Self {
            name: "tekes-drama-storyboard",
            svg: include_str!("../assets/tekes-drama-storyboard.svg"),
        },
        Self {
            name: "tekes-drama-subtitles",
            svg: include_str!("../assets/tekes-drama-subtitles.svg"),
        },
        Self {
            name: "tekes-edit",
            svg: include_str!("../assets/tekes-edit.svg"),
        },
        Self {
            name: "tekes-error",
            svg: include_str!("../assets/tekes-error.svg"),
        },
        Self {
            name: "tekes-error-fill",
            svg: include_str!("../assets/tekes-error-fill.svg"),
        },
        Self {
            name: "tekes-expand",
            svg: include_str!("../assets/tekes-expand.svg"),
        },
        Self {
            name: "tekes-file-added",
            svg: include_str!("../assets/tekes-file-added.svg"),
        },
        Self {
            name: "tekes-file-code",
            svg: include_str!("../assets/tekes-file-code.svg"),
        },
        Self {
            name: "tekes-file-generic",
            svg: include_str!("../assets/tekes-file-generic.svg"),
        },
        Self {
            name: "tekes-file-link",
            svg: include_str!("../assets/tekes-file-link.svg"),
        },
        Self {
            name: "tekes-file-rich",
            svg: include_str!("../assets/tekes-file-rich.svg"),
        },
        Self {
            name: "tekes-file-text",
            svg: include_str!("../assets/tekes-file-text.svg"),
        },
        Self {
            name: "tekes-files",
            svg: include_str!("../assets/tekes-files.svg"),
        },
        Self {
            name: "tekes-filter",
            svg: include_str!("../assets/tekes-filter.svg"),
        },
        Self {
            name: "tekes-flash",
            svg: include_str!("../assets/tekes-flash.svg"),
        },
        Self {
            name: "tekes-flash-fill",
            svg: include_str!("../assets/tekes-flash-fill.svg"),
        },
        Self {
            name: "tekes-folder",
            svg: include_str!("../assets/tekes-folder.svg"),
        },
        Self {
            name: "tekes-folder-open",
            svg: include_str!("../assets/tekes-folder-open.svg"),
        },
        Self {
            name: "tekes-fork",
            svg: include_str!("../assets/tekes-fork.svg"),
        },
        Self {
            name: "tekes-forward",
            svg: include_str!("../assets/tekes-forward.svg"),
        },
        Self {
            name: "tekes-fullscreen",
            svg: include_str!("../assets/tekes-fullscreen.svg"),
        },
        Self {
            name: "tekes-goal",
            svg: include_str!("../assets/tekes-goal.svg"),
        },
        Self {
            name: "tekes-goal-completed",
            svg: include_str!("../assets/tekes-goal-completed.svg"),
        },
        Self {
            name: "tekes-goal-state-active",
            svg: include_str!("../assets/tekes-goal-state-active.svg"),
        },
        Self {
            name: "tekes-goal-state-blocked",
            svg: include_str!("../assets/tekes-goal-state-blocked.svg"),
        },
        Self {
            name: "tekes-goal-state-completed",
            svg: include_str!("../assets/tekes-goal-state-completed.svg"),
        },
        Self {
            name: "tekes-goal-state-stopped",
            svg: include_str!("../assets/tekes-goal-state-stopped.svg"),
        },
        Self {
            name: "tekes-hide",
            svg: include_str!("../assets/tekes-hide.svg"),
        },
        Self {
            name: "tekes-image",
            svg: include_str!("../assets/tekes-image.svg"),
        },
        Self {
            name: "tekes-info",
            svg: include_str!("../assets/tekes-info.svg"),
        },
        Self {
            name: "tekes-job",
            svg: include_str!("../assets/tekes-job.svg"),
        },
        Self {
            name: "tekes-jump-bottom",
            svg: include_str!("../assets/tekes-jump-bottom.svg"),
        },
        Self {
            name: "tekes-lock-fill",
            svg: include_str!("../assets/tekes-lock-fill.svg"),
        },
        Self {
            name: "tekes-logout",
            svg: include_str!("../assets/tekes-logout.svg"),
        },
        Self {
            name: "tekes-mcp",
            svg: include_str!("../assets/tekes-mcp.svg"),
        },
        Self {
            name: "tekes-memorize",
            svg: include_str!("../assets/tekes-memorize.svg"),
        },
        Self {
            name: "tekes-memory",
            svg: include_str!("../assets/tekes-memory.svg"),
        },
        Self {
            name: "tekes-mention",
            svg: include_str!("../assets/tekes-mention.svg"),
        },
        Self {
            name: "tekes-merge",
            svg: include_str!("../assets/tekes-merge.svg"),
        },
        Self {
            name: "tekes-more",
            svg: include_str!("../assets/tekes-more.svg"),
        },
        Self {
            name: "tekes-new",
            svg: include_str!("../assets/tekes-new.svg"),
        },
        Self {
            name: "tekes-new-goal",
            svg: include_str!("../assets/tekes-new-goal.svg"),
        },
        Self {
            name: "tekes-new-project",
            svg: include_str!("../assets/tekes-new-project.svg"),
        },
        Self {
            name: "tekes-no-default-app",
            svg: include_str!("../assets/tekes-no-default-app.svg"),
        },
        Self {
            name: "tekes-note",
            svg: include_str!("../assets/tekes-note.svg"),
        },
        Self {
            name: "tekes-oauth-key",
            svg: include_str!("../assets/tekes-oauth-key.svg"),
        },
        Self {
            name: "tekes-open-external",
            svg: include_str!("../assets/tekes-open-external.svg"),
        },
        Self {
            name: "tekes-pause",
            svg: include_str!("../assets/tekes-pause.svg"),
        },
        Self {
            name: "tekes-pending-task",
            svg: include_str!("../assets/tekes-pending-task.svg"),
        },
        Self {
            name: "tekes-pin",
            svg: include_str!("../assets/tekes-pin.svg"),
        },
        Self {
            name: "tekes-pinned",
            svg: include_str!("../assets/tekes-pinned.svg"),
        },
        Self {
            name: "tekes-plugin",
            svg: include_str!("../assets/tekes-plugin.svg"),
        },
        Self {
            name: "tekes-plugins",
            svg: include_str!("../assets/tekes-plugins.svg"),
        },
        Self {
            name: "tekes-popup-menu",
            svg: include_str!("../assets/tekes-popup-menu.svg"),
        },
        Self {
            name: "tekes-profile",
            svg: include_str!("../assets/tekes-profile.svg"),
        },
        Self {
            name: "tekes-progress",
            svg: include_str!("../assets/tekes-progress.svg"),
        },
        Self {
            name: "tekes-provider",
            svg: include_str!("../assets/tekes-provider.svg"),
        },
        Self {
            name: "tekes-pull-request",
            svg: include_str!("../assets/tekes-pull-request.svg"),
        },
        Self {
            name: "tekes-queued-input",
            svg: include_str!("../assets/tekes-queued-input.svg"),
        },
        Self {
            name: "tekes-quick-panel",
            svg: include_str!("../assets/tekes-quick-panel.svg"),
        },
        Self {
            name: "tekes-quote",
            svg: include_str!("../assets/tekes-quote.svg"),
        },
        Self {
            name: "tekes-read",
            svg: include_str!("../assets/tekes-read.svg"),
        },
        Self {
            name: "tekes-rebuild",
            svg: include_str!("../assets/tekes-rebuild.svg"),
        },
        Self {
            name: "tekes-recall",
            svg: include_str!("../assets/tekes-recall.svg"),
        },
        Self {
            name: "tekes-redo",
            svg: include_str!("../assets/tekes-redo.svg"),
        },
        Self {
            name: "tekes-refresh",
            svg: include_str!("../assets/tekes-refresh.svg"),
        },
        Self {
            name: "tekes-remove-one",
            svg: include_str!("../assets/tekes-remove-one.svg"),
        },
        Self {
            name: "tekes-report",
            svg: include_str!("../assets/tekes-report.svg"),
        },
        Self {
            name: "tekes-resume",
            svg: include_str!("../assets/tekes-resume.svg"),
        },
        Self {
            name: "tekes-review-plan",
            svg: include_str!("../assets/tekes-review-plan.svg"),
        },
        Self {
            name: "tekes-rewind",
            svg: include_str!("../assets/tekes-rewind.svg"),
        },
        Self {
            name: "tekes-router",
            svg: include_str!("../assets/tekes-router.svg"),
        },
        Self {
            name: "tekes-scheduled",
            svg: include_str!("../assets/tekes-scheduled.svg"),
        },
        Self {
            name: "tekes-search",
            svg: include_str!("../assets/tekes-search.svg"),
        },
        Self {
            name: "tekes-send",
            svg: include_str!("../assets/tekes-send.svg"),
        },
        Self {
            name: "tekes-settings",
            svg: include_str!("../assets/tekes-settings.svg"),
        },
        Self {
            name: "tekes-shell",
            svg: include_str!("../assets/tekes-shell.svg"),
        },
        Self {
            name: "tekes-show",
            svg: include_str!("../assets/tekes-show.svg"),
        },
        Self {
            name: "tekes-side-bar",
            svg: include_str!("../assets/tekes-side-bar.svg"),
        },
        Self {
            name: "tekes-side-chat",
            svg: include_str!("../assets/tekes-side-chat.svg"),
        },
        Self {
            name: "tekes-side-panel",
            svg: include_str!("../assets/tekes-side-panel.svg"),
        },
        Self {
            name: "tekes-skill",
            svg: include_str!("../assets/tekes-skill.svg"),
        },
        Self {
            name: "tekes-skills",
            svg: include_str!("../assets/tekes-skills.svg"),
        },
        Self {
            name: "tekes-slash-command",
            svg: include_str!("../assets/tekes-slash-command.svg"),
        },
        Self {
            name: "tekes-sort",
            svg: include_str!("../assets/tekes-sort.svg"),
        },
        Self {
            name: "tekes-steer",
            svg: include_str!("../assets/tekes-steer.svg"),
        },
        Self {
            name: "tekes-steering",
            svg: include_str!("../assets/tekes-steering.svg"),
        },
        Self {
            name: "tekes-stop",
            svg: include_str!("../assets/tekes-stop.svg"),
        },
        Self {
            name: "tekes-subagent",
            svg: include_str!("../assets/tekes-subagent.svg"),
        },
        Self {
            name: "tekes-tab-diffs",
            svg: include_str!("../assets/tekes-tab-diffs.svg"),
        },
        Self {
            name: "tekes-tab-summary",
            svg: include_str!("../assets/tekes-tab-summary.svg"),
        },
        Self {
            name: "tekes-tag",
            svg: include_str!("../assets/tekes-tag.svg"),
        },
        Self {
            name: "tekes-task",
            svg: include_str!("../assets/tekes-task.svg"),
        },
        Self {
            name: "tekes-test-connection",
            svg: include_str!("../assets/tekes-test-connection.svg"),
        },
        Self {
            name: "tekes-think",
            svg: include_str!("../assets/tekes-think.svg"),
        },
        Self {
            name: "tekes-thumbs-down",
            svg: include_str!("../assets/tekes-thumbs-down.svg"),
        },
        Self {
            name: "tekes-thumbs-down-fill",
            svg: include_str!("../assets/tekes-thumbs-down-fill.svg"),
        },
        Self {
            name: "tekes-thumbs-up",
            svg: include_str!("../assets/tekes-thumbs-up.svg"),
        },
        Self {
            name: "tekes-thumbs-up-fill",
            svg: include_str!("../assets/tekes-thumbs-up-fill.svg"),
        },
        Self {
            name: "tekes-time",
            svg: include_str!("../assets/tekes-time.svg"),
        },
        Self {
            name: "tekes-tools",
            svg: include_str!("../assets/tekes-tools.svg"),
        },
        Self {
            name: "tekes-trust",
            svg: include_str!("../assets/tekes-trust.svg"),
        },
        Self {
            name: "tekes-undo",
            svg: include_str!("../assets/tekes-undo.svg"),
        },
        Self {
            name: "tekes-unknown-fallback",
            svg: include_str!("../assets/tekes-unknown-fallback.svg"),
        },
        Self {
            name: "tekes-unknown-tool",
            svg: include_str!("../assets/tekes-unknown-tool.svg"),
        },
        Self {
            name: "tekes-unpushed",
            svg: include_str!("../assets/tekes-unpushed.svg"),
        },
        Self {
            name: "tekes-untracked-folder",
            svg: include_str!("../assets/tekes-untracked-folder.svg"),
        },
        Self {
            name: "tekes-verify",
            svg: include_str!("../assets/tekes-verify.svg"),
        },
        Self {
            name: "tekes-waiting-user",
            svg: include_str!("../assets/tekes-waiting-user.svg"),
        },
        Self {
            name: "tekes-warning",
            svg: include_str!("../assets/tekes-warning.svg"),
        },
        Self {
            name: "tekes-warning-fill",
            svg: include_str!("../assets/tekes-warning-fill.svg"),
        },
        Self {
            name: "tekes-web-fetch",
            svg: include_str!("../assets/tekes-web-fetch.svg"),
        },
        Self {
            name: "tekes-web-search",
            svg: include_str!("../assets/tekes-web-search.svg"),
        },
        Self {
            name: "tekes-write",
            svg: include_str!("../assets/tekes-write.svg"),
        },
        Self {
            name: "tekes-zoom-in",
            svg: include_str!("../assets/tekes-zoom-in.svg"),
        },
        Self {
            name: "tekes-zoom-out",
            svg: include_str!("../assets/tekes-zoom-out.svg"),
        },
    ];
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|icon| icon.name == name)
    }
}
#[cfg(test)]
mod tests {
    use super::Glyph;
    #[test]
    fn full_catalog_is_unique_self_contained_and_tintable() {
        let mut names = std::collections::HashSet::new();
        assert_eq!(Glyph::ALL.len(), 176);
        for icon in Glyph::ALL {
            assert!(names.insert(icon.name));
            assert_eq!(Glyph::named(icon.name), Some(*icon));
            assert!(icon.svg.contains("viewBox=\"0 0 24 24\""));
            assert!(icon.svg.contains("currentColor"));
            assert!(!icon.svg.contains("#000"));
        }
        assert!(Glyph::named("tekes-shell").is_some());
        assert!(Glyph::named("tekes-annotation").is_none());
    }
}
