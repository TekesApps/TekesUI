# 多品牌主题契约

**v0.3.0-proposal.1 · DS-007**

系统定义颜色角色、派生规则与验收标准；消费者提供品牌输入。`#D97757` 只是 `example-clay` 的seed。主参考夹具明确选用 `example-neutral`；它也不是框架暗含的默认品牌。生产消费者必须显式指定配置，缺失即报错。

## 1. 三类输入与唯一来源

```text
批准的基础设计 + 批准的消费者品牌配置 + 版本化派生策略
             ↓ 构建时编译并校验
  品牌四主题语义token + 完整token bundle + receipt
             ↓ 类型化消费
         同一套组件与实际页面
```

`design/tokens/base.tokens.json` 是本提案候选基础设计输入，保留非品牌值、语义关系与组件别名；品牌原始值只写在 `BrandConfig`。`design/tokens/tokens.json` 从显式Gallery参考配置生成，**不可再手改**。消费者编译得到自己的完整token文件，不把蓝色patch撒在各个控件中。

正式采纳仍须回到既有私有设计artifact。共享结构/派生策略由系统维护者批准，品牌配置由消费者品牌维护者批准；二者属于不同受控输入，并以同一发布receipt绑定，不是两个互相争夺的全局真源。

## 2. 最小可运行配置

```json
{
  "schema_version": "1.0",
  "brand_id": "example-blue",
  "version": "1.0.0",
  "seed_srgb": "#2563EB",
  "overrides": {}
}
```

构建命令（Python 3.11+，按根目录requirements安装依赖）：

```sh
python tools/brand.py --brand design/brand/examples/example-blue.json --out /tmp/tekes-blue-1.0.0
python tools/brand.py --brand design/brand/examples/example-blue.json --out /tmp/tekes-blue-1.0.0 --check
```

首次输出目录必须不存在或为空；已有版本不就地覆盖。生成 `tokens.json`、`palette.json`、`receipt.json`，不是Rust运行时。配置ID、版本、seed必填；只接收不透明sRGB `#RRGGBB`。命名色、透明色、未知字段和路径穿越式ID均拒绝。

## 3. 什么跟着品牌变化

| 语义角色 | 用途与变体 |
|---|---|
| `brand-decoration` | 精确保留seed；仅装饰，不用作正文或唯一状态标志 |
| `accent` / `accent-hover` / `accent-pressed` | 链接和文字强调；可读蓝色不必等于原始蓝色 |
| `action-primary-ground` / hover / pressed | 主操作与开启控件的强底 |
| `action-primary-foreground` | 与强底成对的黑/白前景；不是有色品牌变体 |
| `accent-subtle-ground` / hover / pressed | 显式品牌弱背景，适用于具名品牌样式 |
| `accent-subtle-foreground` | 与三种弱背景均配对的可读文字 |
| `accent-outline` | 品牌边界、选中标记；必要非文字信息必须可辨 |
| `action-primary-focus-outline` | 强底上的中性焦点；只调整明暗，不能变成品牌色环 |

组件只用语义/组件token。Button、Link、Checkbox、RadioGroup、Switch及显式品牌选中样式共用同一派生源；普通secondary/ghost不因品牌变化自动变为强彩色。

不随品牌变化：中性表面、正文、辅助字、disabled、普通hover、列表默认选中底、文本选区、错误/警告/成功、语法高亮、diff、图表序列与普通中性焦点。`c-code-keyword` 与 `c-chart-series-3` 从旧accent依赖拆开，维持v0.2原有配色含义。

列表默认仍是中性选中底＋品牌标记；显式 `brand-subtle` 样式才采用品牌弱底。状态或图表需要产品定制时另走语义配色评审，不能偷用seed连带重染。品牌蓝与信息蓝相近时仍依靠标签、形状和位置区分语义。

## 4. 派生算法与四主题

算法ID：`tekes-oklch-constrained-v1`；编译器1.0.0；策略见 `design/brand/policy.json`。

sRGB seed转OKLCH；保持色相方向，近中性输入保持中性。每个角色按light/dark/light-ic/dark-ic的目标亮度及上限色度搜索候选。超出sRGB时在固定L/H下降低C，不做盲目的RGB通道截断。随后量化为8bit不透明sRGB，**用量化后的实际输出值**计算对比度。

搜索为0…1间1001个L候选，色域搜索20步；同距离候选以L再以hex稳定打破平局。固定输入、算法与依赖版本应重生成相同字节；本包验证当前Python环境内的重复性，不声明已验证不同平台浮点实现完全一致。

普通文字≥4.5:1，IC文字≥7:1，必要控件轮廓/标记≥3:1。强底的三个状态同时满足文字与相邻表面要求；弱底有独立文字对。hover/pressed沿该主题批准的亮度方向变化，数值色差至少0.012（Oklab距离），**此阈值只是退化拦截，不是人眼辨识保证**。

检查依赖背景清单，不宣称任意嵌套或透明叠加都安全。组件放入新背景时必须登记新色对。减少动态不改变颜色含义；对比度与动效偏好是独立配置。

这套算法不是Claude或Figma公布的官方派生算法，也不是CSS Local MINDE实现。转换依据与设计取舍见 [重点参考](REFERENCE-POSTS.md)。

## 5. 显式覆盖，而非组件内自由设色

品牌拥有者可在schema白名单角色上提供 `overrides`。每一项必须同时给出四主题值和具体reason；省略角色使用算法结果。覆盖也要通过完整配对、状态方向与退化检查，不静默修复用户输入，更不回退陶土色。

黑白前景由主题策略配对；批准的override仍只允许黑/白。装饰seed与中性焦点等不可作为随意覆写入口。多个品牌之间不建立随意多层继承链：v0.3只允许共享基础＋单层消费者覆盖，减少隐含优先级。

Figma的变量集合/品牌扩展可作为这种结构的工具视图，但不会自动替本系统生成合规色阶。本包适配规划保留语义引用；不能把每个解析值复制成另一套作者输入。[FG-01、FG-02]

## 6. 发布、缓存与运行时切换

receipt记录品牌ID/版本、原始和规范化配置哈希、base/schema/policy/编译器字节哈希、输出哈希及 `theme_contract_sha256`。未获得实际artifact revision时保持null，`approved=false`。哈希证明字节关系，不证明品牌批准或视觉质量。

运行时上下文为 `brand_id+brand_version+theme_contract_sha256` 加独立 `appearance/contrast`。生产由用户/产品显式选择品牌；系统强调色若作为seed，先走同一验证流程。不要在每帧、每个组件内重算色阶。缓存键必须含完整契约哈希，不能只用seed或主题名。

替换必须以窗口/主题作用域一次提交，覆盖浮层、弹窗、图标着色和所有有关组件。失败保留最后一个**已验证**主题并显示错误；首次加载失败无隐含主题可用，阻止创建该界面。不允许旧主题按钮＋新主题链接的半更新。

本包写文件是开发时的逐文件原子替换，并非生产全目录事务；原生实现须写不可变版本目录后原子切换引用。当前未实现原生切换、系统强调色监听或平台缓存。

## 7. Gallery、真实页面与自动门禁

Gallery固定 `example-neutral / example-blue / example-clay × 四主题`，状态、语言、字号、组合风险另按原16项规范覆盖。深链增加brand及契约哈希；未知或未载入版本报错，不悄悄替换为最新。真实消费者增加自己的品牌，不能拿三个样例替代应用验收。

构建门禁包括schema、别名、依赖泄漏、配对对比度、状态方向/退化、生成物新鲜度、receipt和期望矩阵。浅黄、极浅蓝、纯黑白、灰色和固定随机样本用于回归；有限样本不等于所有RGB输入的数学证明。

原生主题切换必须检查：重排不发生非必要变化、焦点/选区/草稿/滚动锚点不丢失、IME不提交、业务事件不重复、菜单与弹窗同帧换色、错误/警告/语法/图表不被品牌污染。L1/L2/L3证据分别保存；目前全部unverified。

## 8. 迁移

`brand-clay` / `p-brand-clay` 保留为显式废弃别名，分别指向 `brand-decoration` / `p-brand-seed`，**不再保证返回陶土色**。新代码禁用颜色名ID。原 `accent` 和 `action-primary-*` ID保留，但其值由配置生成。品牌切换还会改变可见主操作配色，这是proposal内明确的可见破坏性变更。

需要旧外观的消费者先将完整v0.2色对作为迁移候选评审，而非复制几条常量；仍需满足v0.3边界和状态验收。迁移表见 `design/migration-v0.2-to-v0.3.json`。
