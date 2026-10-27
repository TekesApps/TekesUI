# AppWindow public columns and sample assembly — PR52

This commit adds the earlier, separately validated column-interface change on top of the PR's unified Button consumer adapter. Frozen task sources were used; unrelated current UI WIP was not copied. Button Back preparation and the component type assertion from the preceding commit are retained.

AppWindow::from_columns accepts one to three stable unique Column IDs. Each column chooses local Container/original ContentHost or ProcessContainer/fixed factory. Only process columns are registered as workers. Settings assembles its sidebar and form through the same public API. Single-, two- and three-column samples all use from_columns; mixed-columns demonstrates a local trailing view. The legacy AppProfile API is retained for compatibility only.

The development-workspace tests/native evidence from apphost-mixed-columns, apphost-sample-columns and apphost-unified-button were previously verified locally. Their success is not attributed to this draft's old UI base: the PR still lacks the unpublished unified Button implementation and remains blocked for independent compilation. No shared controls code, merge or tag is included.

Fresh validation for this update: the nine identity-bound consumer files match the separate local development workspace exactly. That workspace passed AppHost/Settings 66 tests and sample all-feature/all-target Clippy with warnings denied (development-checks.log). PR checkout formatting and lock metadata checks passed. These do not remove the separately documented missing upstream Button API or establish a successful standalone PR build.
