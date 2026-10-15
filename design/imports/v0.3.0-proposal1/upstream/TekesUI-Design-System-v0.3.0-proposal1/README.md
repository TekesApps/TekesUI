# TekesUI Design System · v0.3.0-proposal.1

**设计提案＋可运行的品牌编译/静态校验工具；不是已实现的Rust组件库。**

本版把品牌从固定陶土色改为消费者输入：`base + BrandConfig + policy → 四主题完整token`。附neutral、blue、clay三组示例，40个通用组件、10项消费层recipe及原16类验收要求。

从 [SPEC.md](SPEC.md) 阅读全规范；品牌重点看 [BRANDING.md](docs/BRANDING.md)，来源重点看 [REFERENCE-POSTS.md](docs/REFERENCE-POSTS.md)，工具边界看 [DESIGN-TOOL-ADAPTERS.md](docs/DESIGN-TOOL-ADAPTERS.md)。[BRAND-EXAMPLES.md](docs/BRAND-EXAMPLES.md) 是生成色表，不是实际Gallery。[AGENT-HANDOFF.md](docs/AGENT-HANDOFF.md) 是可读交接索引。

Jenny Wen 的相关本人文章、原帖与访谈见 [JENNY-WEN-POSTS.md](docs/JENNY-WEN-POSTS.md)；采纳后的真实体验审查见 `design/quality/craft-review.json`。

## 复现

Python 3.11+。本包不执行网络请求，不携带字体或外部资产。

```sh
python -m pip install -r requirements.txt
python tools/generate.py
python tools/seal.py
python tools/validate.py --write-report
python tools/validate.py --self-test --write-report
python tools/context.py --check
python tools/brand.py --all --check
python tools/validate.py --release --write-report
```

最后一条应返回非零：缺少私有设计导出、品牌批准、原生Gallery、实际页面与真实环境证据。`seal.py`只记录本包字节，不批准设计。

单独编译蓝色品牌：

```sh
python tools/brand.py --brand design/brand/examples/example-blue.json --out /tmp/tekes-blue-v1
```

输出目录应为空或不存在。生成物禁止手改；修改seed或四主题显式覆盖后重新编译。品牌schema在 `design/brand/brand.schema.json`；数值策略在 `policy.json`。

本包的Figma规划不是Figma原生导入格式；没有访问Figma/Claude Design在线工作区、修改GitHub仓库或私有artifact。所有代码符号和实际工具键缺失时保留null，所有原生证据仍为unverified。
