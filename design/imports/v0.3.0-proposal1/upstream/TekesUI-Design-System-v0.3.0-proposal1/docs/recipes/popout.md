# PopoutWorkspace · Consumer Recipe

稳定 ID：`agent.popout`。状态：**planned；三层运行时证据均未验证**。可选扩展；不是首轮实施前置条件。

负责人角色：`consumer-ui-maintainer`；实际维护者未指派。所属 crate：`consumer-defined; resolve before beta`。不得放入通用组件 crate 或借此引入模型/网络依赖。

## API 与用法

```text
PopoutWorkspace(session_id, view_id, source_of_truth, on_close)
```

使用：可选消费者能力；按稳定session共享状态，跨窗口恢复焦点和未发草稿。

禁区：不建立第二份业务真源；后台窗口不抢焦点；不宣称已实现。

组合组件：`tui.split-view`, `tui.tabs`, `tui.banner`, `tui.dialog`。

## 状态、主题与证据

领域状态：opening, ready, disconnected, closing, failed。这些不是 hover/focus 的替代；子组件继续逐项满足通用组件状态和组合契约。

主题：light, dark, light-ic, dark-ic；三个 Gallery 品牌夹具全部覆盖，规则见 `../BRANDING.md`。夹具：short, long-title, cjk, empty。具体颜色、尺寸必须来自 `../../design/tokens/`，尺寸超出可用空间时重排，不剪掉操作。

拟定稳定地址：`tekes-gallery://recipe/popout`。当前不是可打开的原生页面。文档与期望用例由 `recipes.json` 生成，不能当作实际组件展示。

参考：OCUI-16；读取范围与证据强度见 `../../research/open-claude-ui.lock.json`。清单级参考不代表复核过源文件实现；适配决定见 `../ALIGNMENT.md`。

真实鼠标、键盘、IME、VoiceOver 和消费者业务 payload 必须独立验收。既有输入、权限、发送偏好和历史分组不得在视觉迁移时静默改变。运行时证据字段：`{"component": "unverified", "application": "unverified", "real_environment": "unverified"}`。
