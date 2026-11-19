# 导航族：共享交互、语义与真实 token

基于 HEAD bc3de79 加全部 WIP。当前刷新 GitHub仍14个开放 Issue、0个全状态 PR，#14更新时间保持2026-10-17T08:15:16Z。沿原 #1/#3/#8处理，不关闭或新建重复 Issue。

## 根因、修复位置与适用范围

1. DisclosureView 在事件间读取过期 disabled，禁用期间可以建立/保留 Space arm，重新启用后误展开。render/pointer_up/key_event/activate现在同步当前模型并复用原 cancel。三项公共回归先失败后通过。适用于全部公开 DisclosureView 消费者；模型的受控展开值不因禁用改变。
2. Disclosure 展开动作使用 Primary强调和Button圆角，违反已有 control_emphasis 的内容/主操作层级以及JSON radius-row契约。共享 View改用既有 Quiet角色、primary-text图标/文字及radius-row；复用Button绘制、状态、动效、焦点和裁剪，不加层、不改颜色表。默认展开动作应保持轻量；真正的主提交按钮仍使用Primary，是合理例外。旧几何测试对白色图标的断言随语义修正为primary-text，保留文字/图标中心与展开前后几何回归。
3. BreadcrumbView缺少整体可用性入口。新增既有View级set_disabled，复用cancel/blur，不改路径；当前末项继续不交互。公开activate/focus的usize::MAX由索引越界/溢出改为拒绝。Breadcrumb是导航链接，Space留给页面，Enter继续激活。
4. 真实原生验收发现公共API之外的宿主问题：Gallery在BreadcrumbView拒绝Space后，仍进入通用Button兜底再次导航。现将已有Link的Space路由规则通过原handles_key判断同时用于Breadcrumb；公共、真实Workbench和原生红绿证据分开保留。不是仅修改组件测试。
5. 真实依赖门禁定位不存在的size-icon-artboard：通用IconView、Disclosure图标槽及Tree展开动作槽曾数值24兜底。分别复用已有size-icon-preview和size-inline-action，移除兜底；不新增token。覆盖全部四处引用（包括Tree回归）。IconButton自己的16pt默认契约保持。Tree既有slot语义和裁剪回归继续通过，本批不据此提升Tree全部状态验收。
6. Gallery复用原矩阵：Breadcrumb焦点移至首次布局之后，补全部祖先禁用；真实Playground复用既有Disabled属性开关；Tabs补hover样例。未新增组件或批量迁移到另一套组合接口。

## 分层验证

公共API：navigation_contract5项，17个必需头部状态+7个登记组合×四主题×三品牌=288案例；hover/pressed/focus必须通过公共输入改变实际生产绘制，不能只写状态名。controls/tokens/design-export共317项通过。公共状态累计89项，仍167项夹具待核验。所有experience仍unverified。

真实接入：CoreText独立消费者26项通过；真实Workbench的Breadcrumb开关点击/禁用以及Link/Breadcrumb Space路由回归通过。原生发现的Space缺陷有独立失败日志和修复后通过日志。工作区、Gallery、独立消费者、token-audit探针严格Clippy通过。

真实token：最终-verified观测文件为当前入口；navigation288、action576、input612、selection2016，均基于当前共享源码重跑，complete=false。原型文件、错误引用门禁失败和中间结果保留，不能当作当前验收。35项能力门禁、6项组合审计测试通过，45组件/184类型分类及269项状态要求保持。

GPU/Gold7：分阶段捕获432张导航头部场景、288张尺寸绑定场景、最终576张（Disclosure/Breadcrumb/Tabs/Tree）。最终矩阵覆盖三品牌、四主题、两倍率、两窗口/文字倍率和三个滚动位置。共1296张是阶段产物，不是1296个独立案例。

仅尺寸token绑定阶段的Disclosure144对PNG逐字节一致。Quiet强调调整的最终Disclosure144张有意变化；相对前阶段Breadcrumb/Tabs288对、Tree144对完全一致。Tree对照是绑定修复后的两个阶段，不能宣称其具有绑定前的GPU证据。捕获后增加的Gallery属性开关和Space路由不改变States绘制，原生与真实事件测试另验。已查看浅色整页Disclosure前后、Breadcrumb焦点/禁用矩阵、深色窄窗大字Tabs中段；这不是全量体验完成。JSON DS、reference与机械Gold7门禁通过。

原生：只操作隔离Gallery。Disclosure鼠标/Return/Space产生true/false/true真实日志，禁用后三种输入无新切换；该截图构建早于后续仅针对导航链接的宿主Space修复，二进制哈希独立保存。Breadcrumb原生修复前点击+Space产生两条导航，最终构建相同操作只一条；禁用后点击/Return/Space不再导航。相关构建哈希和截图分开保存，副本已退出，未操作系统设置或其他应用。原生Tabs全流程本批未验证。

可访问性：最终Gallery系统AX仍只有窗口和菜单，无控件节点。未启动/宣称通过VoiceOver。公共名称或头部绘制不能替代#5生产语义树和#12统一验收。

## 尚未完成的内容状态

Tabs与Disclosure各有loading/error/empty三个必需状态及相关组合，共6个状态+4个组合保留未验证。本批没有将它们改成Header颜色、擅自删除原登记要求或提升成熟度。下一步先核实已有公开组合/反馈/可见性能力能否表达真实消费者内容状态，缺公共契约时在#14依赖入口补齐，不自动迁移其他组件。

原#1仍需全组件/整页收口，原#3仍需其他键盘契约，原#8仍有状态/完整依赖缺口，#5/#12的完整原生/可访问性未齐。本批未测性能，未提交推送/tag/关闭Issue/发布。

证据索引：design/reviews/navigation-family-20261018/receipt.json。
