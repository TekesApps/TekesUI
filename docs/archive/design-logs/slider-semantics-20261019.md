# Slider 共享范围语义与小宿主验收

2026-10-19，HEAD80d6b05 加全部 WIP；最新21个开放 Issue、0个开放 PR。继续 #5 的现有 Slider 接入，以 #13 总览、#14 组合契约、#12 验收为入口。未新增组件、token、状态框架或迁移其他组件。

根因属于公共交互原语守卫与宿主适配：SliderContent 辅助动作直接修改原模型，绕过 SliderView 的当前挂载、可见和窗口状态；默认 Gallery 也缺少真实范围节点。新公共 SliderView::set_value / adjust_value 复用原模型范围、步进、量化和捕获取消；None 拒绝，Some(false) 消费有效但未改值的请求。SliderContent 和 Gallery 统一调用它，原生 Press 不会变成增值。

适用于所有 SliderView 数值动作消费者、现有 SliderContent 和默认 Gallery Slider。合理例外：原模型 Slider::set/key 仍是程序化数据操作，不要求挂载；数值叶子不需要子内容槽位。名称来自实际消费者（独立场景 Volume、Gallery 的 Value 标签），业务范围不是新 token。

复用已有 Slider 小宿主。原生焦点与窗口有效 active 联合判定，失活复用 ContentHost Cancel/focus(None)，即刻发布并拒绝后台 AX 动作。摆放复用 WindowPlacement::Auto；日志与截图保留在稳定回执目录。没有新增 Button 示例。

公共契约：独立新增 API2 项，保留编译后的失败与通过记录；共享233、独立消费者129 项通过。真实接入：最终 Gallery410 项通过、1项既有忽略，新增3项覆盖范围/原模型、边界消费及禁用/失活/旧目标，以及既有 Enter/Space 路由。旧激活辅助函数不代表实际键盘路径有缺陷；真实 Gallery 和 Workbench 测试原本就通过，故未重复修改。此前全工作区1037项通过/1项忽略发生在最后增加这项路由回归之前；最终完整 Gallery 另行通过，不冒充最后一次完整工作区重跑。workspace、消费者及最终 Gallery strict Clippy 和 fmt 通过。

最终小宿主在外接 U2790B、Light、@2x 直接复核：Tab/Shift+Tab、步进键、Home/End、真实拖动、原生 SetValue 量化、相邻编辑器草稿与按键隔离。最小化后 active=false、无焦点节点，辅助赋值未改变95；关闭后应用退出。保存332个实际 Cargo 输入、二进制身份与3份真实生成输入，避免缓存轮换丢失构建证据。因审计元数据更新而废弃的早期轮次未改成通过；最终构建重新进行了原生观察。

Gold7 分别留公共契约、真实接入和 Light 外接2x 的有界体验结果。原 JSON 中性背景、轨道/滑块和独立焦点环保持复用，没有页面补丁。完整 VoiceOver、原生 Increment/Decrement、最小化恢复/跨应用、完整主题/品牌/比例/窗口矩阵、物理 IME 仍未验收。不测性能，不晋级组件或关闭 Issue。

现行 CI 设计门禁通过；组合审计结论经复核后更新 source/anchor，source-only native_acceptance 按原政策仍为未验证，实际证据另列。额外运行的旧 a8 裸基线检查拒绝当前提案 JSON；JSON 与本轮初始 WIP 字节一致，现行 CI 仅执行该旧检查的 self-test，再由 check-design-system 校验现行提案、历史134文件与24张 golden。批准 pin、旧 receipt、golden 未改动，失败日志保留。

[完整记录](../../design/reviews/slider-semantics-20261019/receipt.json)、[最终原生证据](../../design/reviews/component-runs/slider-delivery-20261019/native)、[分层回执](../../design/reviews/component-runs/slider-delivery-20261019/receipt.json)。required-wip-baseline.tar.gz 解包后应用 owned-fix.patch，541个源码文件与 replay-source-sha256.json 逐字节一致；这是9个必需 runtime crate 的公共依赖子集，完整 Gallery 在当前 checkout 验证。隔离 API2 项通过后清理了自己的编译目录，源码与结果保留。最终 App 和普通/哈希命名示例二进制已按新流程回收，公共缓存保留。无暂存、提交、推送、tag、Issue 修改或发布。
