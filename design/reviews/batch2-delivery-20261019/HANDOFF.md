# 第二批审计／门禁交付候选

候选身份：`audit-213cfe8c81b687850aff683a96b5549277fa22cb1c3face7f4cca633dcb529c1`。原提交 `80d6b05649234f953ea452d42e1cbed369fdddbb` 加全部相关 WIP；这不是公开 Git pin。

## 可独立复核的内容

`audit-source/` 封存 304 个原始文件，共约 10.9MB；`audit-source-manifest.json` 逐文件绑定 SHA-256。未改任何原路径源码。快照包含组件登记、45 组件组合矩阵、实际 Rust 声明、七项真实 token 观测及它们的精确消费／脚本输入，并含用户新增 Toolbar 注册要求。

在该目录内运行（只需 Python 标准库，不依赖原 checkout 或 `.git`）：

```sh
python3 scripts/generate-component-capabilities.py --check
python3 scripts/check-composition-audit.py
python3 scripts/test-component-capabilities.py
python3 scripts/test-composition-audit.py
```

四项均已在此隔离快照实际通过。它验证审计／能力证据的自洽与源码绑定，不代替 Rust 编译或原生体验；此快照是审计复核包，不是完整应用发行包。

## 构建门禁的直接证据

`../brand-build-path-20261019-attempt3/` 提供另一独立、可构建的最小双包宿主和未修改的 tokens/compiler 源码。正常 `cargo check --locked --offline -p tekes-ui-tokens` 通过；四种投影篡改保留 hash 后全部被真实 build.rs 拒绝；恢复后通过。收据和每次原始日志独立保留，编译产物已清理。前两次测试夹具准备失败保留，没有算成产品失败或负例通过。

## #6/#8/#13/#14 原条件与交付边界

- #6：共享构建校验和当前 WIP CI 配置已实现，四种普通构建负例实际通过；远端 main 的 #2 CI 修复不能代替新增门禁接受。`proposed-ci.diff` 展示当前 main 与 WIP workflow 的差异。该差异依赖尚未发布的 capability／composition／consumer 脚本和对应公共实现，不能只把 workflow 单文件拣入 main。
- #8：45 组件清单、历史 40 项映射、公开声明反向覆盖及负例可复核。状态与 token 观测仍分别记录有限范围，不自动晋级 experimental 或虚报完整依赖。
- #13：总账保留未完的全库／整页要求。#2 已关闭，#15–#21 已在原公开 pin 独立接受并关闭，不再沿用旧“pin 缺失”阻塞描述。
- #14：组合矩阵、精确源码路径／行号、兼容策略、剩余能力及实施顺序齐备，等待对本候选的独立最终审阅与原 Issue 回填。用户明确 Toolbar 必须允许任意嵌套 Component 贡献到 Root Window 或自身局部目标，Target 与 Placement 分离。该机制尚未实现，内容区按钮行不是窗口 Toolbar。审计完成与未来全部迁移的完成是不同条件。

当前不提交、推送、打 tag、发评论或关闭 Issue；公开交付及其远端 CI 不在此冒称完成。AgentKit `4310937`、037 源包与其他历史证据保持原样；没有新建重复 Issue，没有性能测量。
