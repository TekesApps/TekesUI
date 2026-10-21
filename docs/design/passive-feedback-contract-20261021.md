# Progress / StatusBadge / Skeleton：当前 WIP 契约核验

本轮补齐公共 API 与真实 token 证据，没有修改生产控件、设计 token、公共签名或消费者状态机。依据当前工作树及 JSON Design System；不是新的完整交付候选。

## 根因与共享位置

缺口属于验证覆盖：三个公开被动绘制控件已有实现，但状态账本未绑定实际公共 API 的读取证据。共享实现仍是 `extended::Progress`、`feedback_view::{ProgressView,StatusBadgeView,SkeletonView}` 以及既有语义 palette。

Progress 的 `c-progress-active` 只表示进度，不表示验证成功；完成值 1.0 也不自动等于成功。成功／失败使用独立状态语义。Skeleton 是被动占位，加载生命周期由宿主拥有。本轮没有把不定进度的静态片段擅自增加为动画，也不把已支持的绘制当作完整动画契约。

适用于所有调用这些公共绘制入口的消费者。复用既有裁剪、值约束、palette 和标签前景 API，不新增抽象。被动控件不拥有 hover/pressed/focus；可操作子控件另行验收。

## 分层证据

- `crates/tekes-ui-controls/tests/passive_feedback_contract.rs`：4 项检查通过；有限值与夹取、无效更新拒绝、无面积／不可见几何清除旧布局、实际裁剪、进度与结果色语义分离。严格 Clippy 通过。
- `scripts/audit-passive-feedback-tokens.py`：216 个实际读取场景，四主题、三品牌、完整／部分裁剪。Progress 96，StatusBadge 96，Skeleton 24。标记 `complete:false`，不声称全分支 token 闭包。
- 真实独立消费者：原有 `content_status` 两项检查通过，包含实际 Progress 加载接入与宿主错误／空内容、隐藏编辑器及保留草稿的行为。不是重做一套消费者状态机。
- Gallery：原有 StatusBadge 实际成形标签的 48 个主题／品牌／状态组合对比度检查通过；现有 Toast／Progress 大字测量检查通过。这里检查的是 Scene 和文字几何，没有新增 GPU 或原生图像证据。
- 44 项能力治理、8 项组合审计、生成投影及格式检查；生成公共绑定单独编译留证。

证据目录：`design/reviews/passive-feedback-contract-20261021/`。初次新增测试以精确浮点等号比较 60.000004 与 60.0 失败，改为有界误差断言后通过；保留原失败日志，不将其描述为生产缺陷。

## 账本边界及下一步

只将 Progress 的 rest／indeterminate、StatusBadge 的 rest／loading／success／error、Skeleton 的 rest 标为公共 API 已验证，fixture 和 experience 保持未验。没有更改注册成熟度。

仍须补齐：

1. StatusBadge 的 unverified／cancelled 真实消费语义与标签接入，确认能否复用 Info 与调用者标签，而非立即扩展公共 enum 或增加 token。
2. Progress 的 loading／error／success 按宿主组合契约验收；不得用 100% 代替成功或失败，也不能仅凭子控件的颜色测试完成系统问题。
3. 上述最终消费者的真实 GPU、外接屏原生、必要 AX，以及与 ClientKit 同内容、主题、窗口、字号和倍率的体验对照。当前结果不证明 Gold7 整体通过。
4. 同一最终交付版本的全工作区、远端 CI 与独立审查仍另行收口。

本轮未启动新窗口，VoiceOver 保持关闭；本轮测试可执行文件已按独占缓存锁和完整打开文件清单检查后删除，共享依赖、源码、原始日志与观察数据保留。原有集合生命周期宿主未改动，仍等待真实系统重新激活的最后两步。未测性能，未提交、推送、合入、打 tag 或关闭 Issue；没有改动 AgentKit 冻结输入。
