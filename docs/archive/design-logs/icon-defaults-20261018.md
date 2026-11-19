# IconButton 共享默认几何与真实 token

以全部 WIP 为基线；最新 GitHub 仍14个开放 Issue、0个全状态 PR。原 #1/#8 继续处理，不关闭或新建重复 Issue。

## 根因与共享修复

现有 JSON component-contracts 已规定 default_hit=size-icon-button、compact_hit=size-icon-button-compact、icon=size-icon-standard、radius=radius-control。此前没有公共自然尺寸入口，Gallery 用44×40或220pt宽样例代替；未指定图标大小时通用 IconView 回退24pt，与 IconButton 的16pt契约不同。圆角数值虽相同，依赖却错误地沿用 c-button-md-radius。

IconButtonView 新增 measure(compact)，读取已有32/28pt token；render_icon 的非正数或非有限尺寸使用已有16pt token，正常正数仍为显式尺寸。自然尺寸供布局采用，render 保留调用者明确 frame，不偷偷放大/缩小。既有通用 IconView 的24pt自然画板契约保持。

共享 Button 内部绘制路径增加 crate-private 的圆角输入，保留原有 ButtonSize 默认圆角。IconButtonView 传入 radius-control；没有新建状态机、公共样式参数或抽象层。所有交互、动效、裁剪和焦点继续复用原有 Button 实现。适用全部 IconButtonView 消费者；Button、Disclosure 等继续自己的语义半径，是合理例外。

Gallery 的已有 Playground 和 States 使用同一公共 measure 和默认图标尺寸；不为页面单独画32pt按钮。显式56×48 frame/20pt图标的独立消费者案例保留，说明已有布局覆盖路径没有破坏。紧凑28pt只做公共 API/消费者核验，没有扩大 Gallery 变体范围。

## 分层证据

公共 API：图标默认尺寸红绿回归从24pt失败转为16pt通过。7项 action 契约通过；token-audit 的每个 IconButton 案例断言实际读取三个语义几何 token，并且不读 c-button-md-radius。普通/紧凑自然尺寸、居中及显式 frame/图标尺寸均核验。controls+tokens+design-export共312项通过。

真实接入：CoreText 独立消费者25项通过；真实 Gallery Workbench 的自然 frame 与图标尺寸回归通过。工作区、消费者和 token-audit 探针严格 Clippy 通过。

真实 GPU/Gold7：IconButton States 144张和普通 Button States/Variants 288张，覆盖三既有品牌、四主题、两倍率、两窗口/文字倍率和三个滚动位置。IconButton144张为预期几何变化，旧图独立保留；Button288对 PNG 逐字节相同。查看浅色普通整页，状态按钮现在保持紧凑方形、图标居中、焦点完整；既有加号奇偶尺寸/@1x/@2x双轴像素对称测试通过。机械 Gold7、reference 与 JSON DS门禁通过。能力元数据在捕获后刷新，未进入捕获 States/Variants 的 Docs 摘要路径。

原生交互：用户授权仅 Gallery，使用当前构建的隔离 Gallery 副本；鼠标点击、Return、Space产生三条真实激活日志；整体禁用后点击与两键不新增激活；重新启用后可点击，Tab/Shift-Tab恢复可见键盘焦点。截图与二进制哈希单列，副本验收后退出。未操作其他应用或系统设置。

可访问性：本批实读系统 AX 仍只有窗口和菜单，没有 IconButton 节点。页面上的 accessible name 文本和公共模型名称不能替代导出语义树。#5 生产 Gallery 接入与 VoiceOver朗读/导航继续未完成，未启动或宣称通过 VoiceOver。

运行时 token 探针均按当前源码重跑：action576、input612、selection2016；依赖集合仍有界 complete=false。公共状态核验累计72项，184项夹具待核验；不提升实验性成熟度。组合审计仅重新核对实际源位置和哈希，内容容量/协议适配结论不变；能力门禁34项、组合审计6项通过。

本批不测性能、不提交推送/打tag/关闭Issue。下一批继续 Disclosure/Breadcrumb/Tabs 导航族的状态和真实 token 依赖；全量 Gallery AX迁移不自动扩入本批。

证据索引：design/reviews/icon-defaults-20261018/receipt.json；各类证据与修改前源快照分开保存，既往批次不改写。
