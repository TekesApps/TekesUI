# #7：消费方 BrandConfig 离线接入

2026-10-18，活跃 Gallery checkout 的 HEAD bc3de79 加全部 WIP。实时核对开放 Issue 仍14项、PR为0；#7与#14更新时间未变。本批证据 `design/reviews/consumer-brand-20261018/receipt.json`；总目标仍 active。

## 根因与共享修复

根因属于公共消费契约与交付边界：原品牌编译器可在独立源目录合并，但 tokens/build.rs 只读取依赖 checkout 的默认配置；Palette 又会把未知品牌索引静默回退到基色。

复用既有 `merge.merge(root)`、BrandConfig schema、版本化 policy、颜色依赖图、完整投影与 Rust verify_inputs。新增显式 `scripts/compile-consumer-brand.py --config --output`，只在消费方新目录暂存既有作者/编译输入并替换该副本的配置，不编辑依赖 checkout 或原始导入包。拒绝已有路径（包括失效符号链接）、checkout 内输出、无效配置/角色/schema。生成环境使用提案 requirements；库构建和运行时不启动 Python。

`TEKESUI_DESIGN_ARTIFACT` 是构建时显式绝对目录。公共 `verify_consumer_inputs` 先验证 artifact 原始输入与投影摘要，再要求设计/API格式及除消费方 config 外的全部输入与当前依赖一致；embedded config 必须匹配其文件。缺失、旧版本、不兼容、相对路径都明确失败，不读取默认 artifact 兜底；Cargo 同时追踪 artifact 和依赖输入及环境变量。支持方式是完整精确 Git revision + 离线 artifact，当前全部 WIP 用显式开发 path 验证；未声称远端 commit 已含这些未提交变更，不发布 crate。

`Palette::new` 复用已有预设查询返回 Option，未知品牌在 `color` 入口也拒绝，现有字面量不能绕过；未知角色仍返回 None。默认 Palette::from(theme) 采用消费方编译品牌；显式 Gallery 预设仍是预览能力。身份复用既有 proposal::BRAND_ID，同处补全从同一配置生成的版本/seed元数据，没有新增 palette/provider 层。原迁移单测改读本次构建选中的投影，使自有品牌可接受相同的508通道与完整词汇核验。

适用范围为所有使用默认 Palette/Theme 的真实控件，消费方无需传局部颜色。合理例外：同进程动态品牌加载/热切换不在本方案内；多消费项目分别编译。artifact 与精确设计版本匹配，升级依赖必须重新生成。存储目录与发布打包归消费方。说明和独立模板见 examples/brand-consumer/README.md。

## 分层证据与边界

- 公共API：2项artifact兼容性/缺失输入/投影篡改/绝对路径回归；Palette四主题未知品牌/角色拒绝测试；控件/tokens/design-export默认304项通过；显式自有apricot artifact下tokens22项通过。
- 独立真实消费：脚本在仓库外创建2个独立Cargo workspace，各持有自己的BrandConfig，依赖同一完整WIP；不同seed、四主题各12个运行时角色、主动作rest/hover/pressed互异；真实Button绘制断言颜色确来自所选Palette。101个不依赖品牌的颜色角色与默认品牌逐主题比较一致。错误配置/角色/schema、输出碰撞、失效符号链接、checkout内输出，以及真实Cargo缺失artifact失败均核验。CPU模式2项、GPU模式2项通过。CI既有scripts/test-*.py入口持续发现此脚本，无需另造CI管线。
- GPU/Gold7：2消费者 × 4主题 × @1x/@2x共16帧，实际Button Primary/Quiet/Panel × rest/hover/pressed/disabled，共192次真实组件绘制，CoreText文本与实际GPU路径。抽查apricot浅色@1x和seaglass深色IC@2x，注意力、可读性、中性基础与共享几何保持，品牌只派生允许角色；未改任何JSON设计值。没有把该小场景推断为全Gallery逐页验收。
- 原生/可访问性：本批是离线API及离屏组件接入，没有操作外部消费者窗口；不占用“仅Gallery”的桌面授权，不更改系统设置。真实原生交互、系统IME、VoiceOver及G4时间过程未在本批重新验收，继续保留原Issue的统一验收边界。

严格workspace、token-audit和独立品牌消费者Clippy、fmt、DS、Gold7机械、reference alignment、纯Rust、能力生成及32项治理通过。输入612/选择2016例有界token观察基于当前源实际重跑，旧批次证据不改写；完整组件token依赖与201项状态夹具仍待核验。

#7 记为已实现待验收。未提交、推送、tag、关Issue、发布或测试性能。下一项继续 #14 的当前45组件组合能力矩阵，核验公共API、真实接入和未完成依赖，避免沿用旧基线的阻塞结论。
