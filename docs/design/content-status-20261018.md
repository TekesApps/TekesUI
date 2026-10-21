# Tabs / Disclosure 内容状态与共享输入隔离

基于 `bc3de79` 加全部现有 WIP。此次处理 #14 的调用方组合能力及 #8 的 6 项内容状态、4 项焦点组合，验收继续归入 #12；未关闭任何 Issue。

## 根因与修复范围

| 根因 | 共享修复位置 | 适用范围与例外 |
|---|---|---|
| active 子树虽已停止绘制，旧焦点/捕获仍可直接查到隐藏目标 | `ContentHost::reconcile_active_inputs` 在事件、焦点、AX 动作与 render 边界核对 active 祖先链；取消旧输入，清理隐藏祖先中的模态 | 所有保留但隐藏的 Content 子树。Dialog 自身退出绘制仍允许，隐藏祖先则隔离整棵子树；几何变化仍须重新布局 |
| Component 的动态文字服务不能直接传给原 View 的定长参数 | 为 `&mut T where T: ButtonTextRenderer + ?Sized` 转发全部原服务 | 独立消费者复用原测量、IME host、时钟、帧请求和倍率。未替换原 View 签名；当前消费者通过，未宣称所有下游自定义引用 impl 均无冲突 |
| Gallery 扩展组件提前接收宿主页签激活，部分 body adapter 还消费无关 pointer-up | Workbench 先处理自身页签；模态/弹出作用域之后，优先释放宿主标题按钮 | 所有扩展 renderer 的 Docs / Playground / States；已回归模态阻断，不改组件状态机 |

没有新增内容状态协议或设计 token。调用方组合复用 `active`、`StackContent`、`TextFieldContent` 及 Progress / Banner / EmptyState 原 View。Tabs/Disclosure 原 View 继续负责头部交互；内容状态不会强制改变选中页或展开值。保留的真实输入框失焦时沿用既有策略：完成当前可见组字，拒绝隐藏期间迟到的输入，返回后保留草稿和选区，并可撤销组字。

## 分层验证

- 公共 API：共享控件 319 项测试通过；新增隐藏焦点/旧捕获及嵌套模态回归具编译成功的修复前失败日志。动态文字引用在外部 crate 中实际传入生产 View / ContentHost。
- 真实消费者：独立 crate 28 项测试通过；2 类内容 × 5 个状态/组合 × 4 主题 × 3 品牌共 120 项断言。头部原 288 项观测合为 408 项有界 token 证据；没有把焦点组合冒充单状态或完整依赖。
- GPU / Gold7：最终 480 张真实文字内容场景，另有两组件 States 路线的 288 张整页捕获，覆盖 4 主题、3 品牌、1×/2×、窄宽度大字号与宽宽度。核对窄宽度换行、错误语义与焦点分离、裁剪和滚动末端。GPU 图像数量不代表完整体验通过。
- 原生 Gallery：显式 `composition-review` 构建可经真实搜索、页签和滚动看到内容及焦点组合。Disclosure 页签故障留有前后截图；全部扩展 renderer 的指针切换回归及既有模态排他测试通过。只操作 Gallery，未改系统设置。
- 可访问性：公共语义/动作及选区自动测试有证据；原生 AX 仍仅暴露窗口/菜单，控件树、准确 Tabs 等角色及 VoiceOver 未通过。不以静态矩阵证明真实内容的原生输入交互。

默认 Gallery 未增加组合依赖或迁移场景；新内容矩阵仅在测试或 `composition-review` 功能中启用。内置 Tabs/Disclosure Content 适配器依旧缺失，调用方包装证明现有公共能力可用，不等同内置支持完成。Progress 的完整忙碌语义、加载动效体验等也未据此晋级。

## 当前状态与后续

能力清单累计 95 项公共状态验证、161 项夹具待核验，仍保留全部 269 项状态要求。完整 token 依赖、其他组件族、生产 Gallery AX、物理 IME 和统一最终体验验收继续开放。下一步按组件族核验 Select / ComboBox / CommandPalette 剩余状态及公共依赖，优先复用现有输入、菜单和焦点能力。

完整源文件、日志、token 观测、截图、源码与 WIP 保护散列见 `design/reviews/content-status-20261018/receipt.json`。未测试性能，未提交、推送、打 tag、关闭 Issue 或发布。
