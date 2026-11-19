# CommandPalette 公共调用者焦点交接（#4）

基于 80d6b05 与活跃 checkout 的全部 WIP。根因属于公共组件契约：Gallery 已修复隐藏输入释放，但公开 CommandPaletteView 无法保存调用者，独立消费者必须自行重写关闭后的焦点交接。优先复用既有 view、TextFieldContent 与 ContentHost，没有新增层、token 或状态机。

新增 `open_from(model, invoker)` 在本轮打开时保存调用者、请求宿主将焦点转入查询；重复调用不清空正在输入的查询，也不替换初始调用者。`take_restore_focus(model)` 在关闭后仅返回一次调用者；宿主继续使用现有 mounted/enabled 校验。整组件 blur 与窗口失活丢弃交接，避免焦点已离开时抢回。原 open 和触发器接口保留，不携带旧一轮的恢复目标。

真实接入：Gallery 的打开/关闭入口使用公共交接，保留已挂载但被裁切调用者的几何修复，拒绝禁用和卸载目标；独立命令消费者原 view + TextFieldContent 也使用同一接口。适用于 CommandPalette 的 Escape、有效提交、外部关闭及候选项动作。MenuView、DialogView/ContentHost 已有自己的调用者/模态契约，是合理差异，不另建全局抽象。

验证分层：

- 公共控件 235 项通过；新增交接测试覆盖原调用者、重复打开、查询保留、Escape/Enter/取消的一次性恢复、blur/失活、不带调用者旧 API 重开及 disabled。
- 独立真实消费者 132 项通过；新增 Escape/提交/取消后无 TextInputState、真实触发按钮焦点以及迟到提交不重开的回归。原有查询/IME/响应更新测试继续通过。
- Gallery 412 项通过、1 项既有可选忽略，包含禁用/卸载/裁切调用者、组字取消优先及实际 GPU 场景。两侧 all-target strict Clippy、15 项 CoreText shape 检查通过。
- 最终小宿主在外接 U2790B、2x 使用 CUA 原生按键与 typeText；明确观察非空查询后 Escape、提交、外部点击均恢复 Find a command，后续实际输入 late 不重开。完整 AX、实际 Metal 截图及输入前后证据见本轮 receipt。
- Gold7 仅核对本次交接改动的 G1–G3 既有表面/边界、G5 输入释放与 G7 真实组件接入。G4 完整动效、物理 IME、VoiceOver、全部主题/品牌/倍率及整页接受没有因此升级，G6 不适用于本命令场景。七份 token 探针均实际重跑，不只重绑 hash。

最终小宿主代码为原生 host 增加 command 模式，复用既有桥；试验脚本未作为交付能力保留。前两轮空字符合成事件、窗口中途失活的失败日志保留，未计通过；最终采用真实原生操作留证。最终 [receipt](../../design/reviews/component-runs/command-focus-cua-20261019/receipt.json) 已接受并清理二进制，源代码和全部证据保留。#4 的物理 IME 和 Gallery 全场景最终统一验收仍需按原验收边界处理。

AgentKit 4310937 与旧 037/e4 源码封存包没有覆盖；本批共享新输入是另一候选，尚未作为 AgentKit 依赖提交给消费者。未测性能。


## 最终物理组字补验

`command-ime-final-20261019` 基于新增通用只读输入诊断后的当前 WIP。公共 controls lib 238 项、独立消费者 136 项通过。外接 U2790B @2x，用户以系统拼音物理输入 n、i，原生日志记录 marked `[0,2]`；第一次 Escape 清空组字，同时活动窗口仍发布 Command query。切回 Codex 后的失活关闭单独记录，未混作 Escape 结果。重新打开空查询后普通 Escape 结束文本输入并恢复 Find a command；用户继续物理按 x，确认没有重开。日志和用户观察相符。

本轮实际 Metal 关闭态图像证明没有残留隐藏查询；Gold7 仅晋级 G5 的本次组字／焦点生命周期，不据此晋级其他视觉、动效或 VoiceOver 条件。Enter／outside 的既有原生收据与当前自动回归分别保留。第一轮因工具门禁修复和未收到物理结果而明确 discarded，未作为接受。最终轮次已通过分层登记并清理产物，源代码和证据保留。#4 仍需交付身份及独立最终复核，未自动关闭。
