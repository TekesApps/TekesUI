# AgentTaskBoard · Consumer Recipe

稳定 ID：`agent.task-board`。状态：**planned；三层运行时证据均未验证**。可选扩展；不是首轮实施前置条件。

负责人角色：`consumer-ui-maintainer`；实际维护者未指派。所属 crate：`consumer-defined; resolve before beta`。不得放入通用组件 crate 或借此引入模型/网络依赖。

## API 与用法

```text
AgentTaskBoard(tasks, artifacts, evidence, on_action)
```

使用：可选消费者看板；任务、产物与验证结果分别呈现。

禁区：不把执行完成显示为验证通过；不嵌入agent执行器到通用crate。

组合组件：`tui.grid`, `tui.list`, `tui.status-badge`, `tui.disclosure`。

## 状态、主题与证据

领域状态：empty, running, complete, failed, unverified。这些不是 hover/focus 的替代；子组件继续逐项满足通用组件状态和组合契约。

主题：light, dark, light-ic, dark-ic；三个 Gallery 品牌夹具全部覆盖，规则见 `../BRANDING.md`。夹具：short, long-title, cjk, empty。具体颜色、尺寸必须来自 `../../design/tokens/`，尺寸超出可用空间时重排，不剪掉操作。

拟定稳定地址：`tekes-gallery://recipe/task-board`。当前不是可打开的原生页面。文档与期望用例由 `recipes.json` 生成，不能当作实际组件展示。

参考：OCUI-17；读取范围与证据强度见 `../../research/open-claude-ui.lock.json`。清单级参考不代表复核过源文件实现；适配决定见 `../ALIGNMENT.md`。

真实鼠标、键盘、IME、VoiceOver 和消费者业务 payload 必须独立验收。既有输入、权限、发送偏好和历史分组不得在视觉迁移时静默改变。运行时证据字段：`{"component": "unverified", "application": "unverified", "real_environment": "unverified"}`。
