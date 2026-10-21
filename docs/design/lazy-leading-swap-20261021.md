# Leading 懒启动、center/trailing 交换与导航分离

状态：2026-10-21 实现并通过原生实际点击；替代此前以列排列演示导航的窗口方案。

## 当前行为

- 三列 profile 默认 leading 关闭。Panel 只保留声明和空 UI 占位，不启动后台 producer、子进程或 GPU；可见布局首次挂载时才调用既有运行器启动。冷启动是 Host + 导航 + center/trailing 两个 View，共四个进程。
- 首次点 leading 开关才启动第三个 View，之后隐藏/恢复保留同一 PID 和业务状态。恢复后的输入仍等待有效几何。单列 profile 的启动行为不变。
- 右上窗口 toolbar 依次注册“交换 center/trailing”“trailing 开关”。交换使用公共 `tekes-swap-horizontal` 图标；trailing 隐藏或导航切换未完成时交换按钮禁用。R 与按钮执行相同的本地交换。
- 交换只改变 Host 的物理槽位到模块实例映射，leading 不参与交换。原内容、计数、模块 toolbar 和命令所有者一起移动，不重建进程、组件或业务状态。窗口级开关固定位置，不随模块移动。
- 交换、显隐不发送 NavigateRequest，不修改导航 entries/cursor/revision，也不写 nav.json。它们仅是当前窗口内的布局状态；本轮未增加跨启动布局持久化。

## 导航演示调整

旧 `panel-layout` 路由不再用于当前演示。新的 `demo-document` 路由只记录 document_id，窗口标题显示选中文档；文档前后导航不会改变列排列。此路由是原型选择状态，不代表 Kit 业务内容的完整联合事务。

新命名空间为 `three-panel-documents`，文件目录是 `~/Library/Application Support/TekesUI AppHost Prototype/navigation-documents/`。旧 `navigation/` 下的布局导航夹具完整保留，不静默覆盖或伪装迁移。空的新历史仍由导航进程通过正常执行/确认顺序建立文档 A/B/C；重开后恢复已有文档，不追加历史。

文档标题先发布，再等可见 View 的有效几何和实际 GPU 完成，才发送执行成功或恢复确认。布局交换保持内容输入/toolbar arm 取消和几何门控，不绕过窗口激活检查。

## 验收

入口与证据位于 [验收目录](../../design/reviews/lazy-leading-swap-20261021/acceptance.json)：

1. 原生冷启动只有 demo-second、demo-third 两个 View，demo-first 不存在。
2. center 的英文模块计数加一后交换：英文内容、计数 1、`Increment (1)` toolbar 一起移到 trailing；原 Unicode 模块移到 center。
3. 首次打开 leading 后新增一个 demo-first PID；隐藏再打开复用同一 PID。
4. trailing 隐藏时点击交换不执行；恢复按钮始终可见。
5. 上述全部布局操作前后，新 nav.json 字节一致。之后单独点击后退选中文档 B，布局仍维持交换后的顺序。
6. 最后重开 App，leading 再次默认不启动，已有文档 B 恢复且历史不追加。

四项 AppHost 定向测试、两项图标目录测试及三 View 真实 GPU/IPC 回归分层留证。公共 runtime 仍有其他会话的并行 WIP；完整依赖严格 Clippy 的结果单列，不将未通过检查报告为通过。此轮不切换到中间态公共 V2 View/ProcessContainer，不提交或发布。
