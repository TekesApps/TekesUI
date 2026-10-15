# 迁移到v0.3

从v0.2的固定品牌token迁到 `base.tokens.json + BrandConfig + policy`。`tokens.json`变为生成物；保留原消费文件路径。明确选择品牌ID和版本，禁止缺省使用clay。

`brand-clay` / `p-brand-clay` 仅作废弃别名，新代码分别使用 `brand-decoration` / 语义角色。链接和主操作已有ID保留，但按品牌派生；普通中性表面与语法/图表不随品牌变化。

核对全部实际页面后再批准；品牌输出不能靠数学对比度通过就发布。Figma/Claude视图的修改进入staging差异，并回到既有artifact评审，不直接覆盖已发布token。源码映射不明确的组件保持planned与null。

逐项机器可读变更见 `../design/migration-v0.2-to-v0.3.json`；详细契约见 [BRANDING.md](BRANDING.md)。历史v0.1→v0.2迁移记录保留如下。

---

# v0.1 → v0.2 迁移

这是提案版本内的可见设计变更，不是已发布稳定API的无影响patch；设计真源采纳后才能进入代码迁移。

## 保留

原有全部token ID和38个通用组件ID均保留；四主题ID、三层证据、原生平台边界、无阴影和中性focus不变。新增2个通用组件、10个消费层recipe并不表示已经实现。旧artifact原始导出未取得，其真实schema round-trip仍未验证。

## 需要处理的可见变化

`radius-control` 28→8、`radius-input`16→8、`radius-panel`12→10、`radius-floating`14→12。原来用control token做胶囊的地方必须显式迁移到 `radius-pill`，不能继续依靠相同旧名称维持原来外观。

`size-icon-button`28→32；需要紧凑尺寸的场景明确采用 `size-icon-button-compact`。新增Button XS/SM/MD/LG类型化尺寸，不将列表密度直接当作按钮variant。

主画布、侧栏及surface映射调整；品牌陶土与可读accent、主按钮色对分离。所有消费者必须重测selected+hover、focus+pressed、代码选区及浮层叠色，不能只接受新背景截图。

ui-caption10→11、ui-secondary11→12。新增chat/composer和Button文字profile；长度、截断、基线与200%文字需要重新验收。侧栏默认288、阅读列最大768会改变断行，不允许只更新像素容差掩盖差异。

时长与spring见motion.toml。新动效不会自动授权按压缩放、逐字模糊或持续背景动画；减弱动态直接终态。

## 迁移顺序

先在作者artifact采纳；再冻结设计版本与原始导出摘要；原子导入token、schema、motion、registry和规范；生成Rust常量；按组件和实际页面逐项替换；最后重新取得L1/L2/L3证据。新鲜度同时对比已提交生成物和独立approved-design-head。

保留原用户发送设置、授权、分组、草稿、选区、焦点和撤销栈。视觉迁移不得静默执行文档里讨论的历史会话归并、模型清单替换或后端协议改变。发现不兼容应提交行为变更评审。

完整字节级token差异见 `../design/migration-v0.1-to-v0.2.json`。`changes`保存设计理由，`token_changes`是原候选值与本候选值的精确比较；引用仍需解析后的影响分析。源revision/export hash未知项继续为null，不得用本包SHA代填。
