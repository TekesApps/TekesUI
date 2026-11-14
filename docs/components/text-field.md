# TextField

TextField 是单行文本编辑组件。它管理文本、选区、撤销及组字；表单保存和业务校验由消费者组织。

## 最小组合

```rust
use tekes_ui_controls::composition::*;

let field = TextFieldContent::new("Document title", "A quieter workspace");
let host = ContentHost::new(Content::new(ComponentId(2), field)).unwrap();
```

与现有 `examples/composition-consumer/src/search_field.rs` 中的普通编辑器采用相同正式入口。
AppHost 提供原生文本服务与窗口级键命令，不要在页面外再安装一套文本输入代理。

## 属性与状态

- `new(label, value)` 创建带标签的单行 Field。
- `surface` 是该实例的正式 TextSurface，承载文本输入与编辑状态。
- `viewport` 指定内容视口策略；它不等于外层 Container 的分配大小。
- `show_metadata` 控制字段元信息的呈现。
- `tab_policy` 默认 Navigate；`submit_policy` 默认 None，可选择 Enter 或 CommandEnter。
- `scrollbar(visible)` 仅控制横向溢出指示，不关闭文字滚动或插入点显露。
- `inline(name, value, width_em)` 用于紧凑行；`read_only_content(name, value, role)` 用于可选择的只读正文。
- 使用 Host 的 `get(id)`、`component::<TextFieldContent>()` 读取实例；需要更新时通过对应正式编辑/同步合同处理。

声明尺寸、最终布局、caret、选区、逻辑焦点与键盘焦点装饰是不同数据。Inspector 必须分别展示，不能用“被点击”推定所有状态。

## 输入与组合

原生 TextCommand、剪贴板、选择与撤销由正式 ContentHost 路由。输入法组字期间不能因文档面板刷新而提交或重建编辑器。
单行编辑器默认不需要显示横向滚动条；内容溢出时的编辑滚动仍与滚动条显隐分开。
放入 Form 或 Stack 时复用原实例和稳定 ID，切换文档说明不应重置草稿。

## 外观

Field 使用共享语义颜色和原轮廓上的焦点提示；错误与禁用各自独立。
只读正文应选择 Content presentation，不把带边框的 Field 当成长文阅读容器。ScrollArea 不替子组件擦除边框。

## 限制

这里记录单行 Field 的主要消费路径。多行文字、数字解析及只读富文档有独立合同；不能根据共同 TextSurface 推定它们所有键盘策略相同。

公共定义见 `crates/tekes-ui-controls/src/composition/adapters.rs`，键策略见 `composition/key.rs`。启用 `host.set_action_events(true)` 后可消费正式提交动作；外部值发布使用 `surface.apply_external_value(...)`，须处理其返回值，不能当成用户提交。本批仅核对源码，未编译文档片段。
