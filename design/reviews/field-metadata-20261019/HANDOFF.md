# Field 元数据候选复现入口

源码身份：`203d38ff425a272bb917a54ec89563f0a9c0999b1f09bfea2ebb96359796305b`。
压缩包：`tekesui-field-203d38ff425a.tar.gz`，SHA-256：`61c838c76672bd6be81d7ce484ea2f628eba65e28acd8b5c7506f31951b23bd0`。
100%源文件已逐项校验，见 manifest.json；全新解压目录的全部本地Cargo manifest均在包内。
独立解压回放3项公开测试（含48个palette/role/message案例）和142项消费者测试通过。
临时解压目录及该轮本地重建产物已清理，源码、压缩包、证据和注册表依赖缓存保留。

## 范围

仅修复既有 FieldView 的长标签/必填星号布局与字体角色；独立消费者组合原 TextFieldContent，
覆盖正常帮助、错误和禁用三组输入。未新增组件、token或抽象；普通表单/只读content语义不混淆。
本包以已验NumberField源码为底，叠加全部本轮增量；并行Scene3D/TickingNumber未纳入本包验收，WIP原样保留。
此前AgentKit #23 的2f0902d9导出不变，不要求对方重复构建已交付输入。

## 从包复现

```sh
tar -xzf tekesui-field-203d38ff425a.tar.gz
cd TekesUI
cargo test --locked --offline -j 2 -p tekes-ui-controls --test field_contract
cargo test --locked --offline -j 2 --manifest-path examples/composition-consumer/Cargo.toml --tests
```

当前环境为macOS/CoreText/Metal。首次无注册表缓存时需先获取lockfile依赖；下载缺失不代表产品失败。
请设置专用CARGO_TARGET_DIR、debug=0和incremental=false，并复用既有单线公共缓存/锁。
原生入口为同一consumer的native example之field模式；本工作树已完成标准轮次
field-metadata-native-20261019，现已关闭清理。若需复验，请创建新轮次并保留旧收据，不复用已完成轮次。

## 验收边界

公共API、真实接入、96帧最终GPU、严格Clippy与隔离源码门禁已通过；前后对照保留。
外接屏12个活动检查点已通过：Tab/Shift+Tab、真实指针、AX更新/Backspace、禁用隔离及360pt窄窗焦点/错误布局。
原生与有界Gold7证据分别登记；轮次accepted/cleaned，窗口关闭、专用二进制清理。
这只接受本轮Field元数据修复，不据此提升冻结候选、晋级组件成熟度或关闭总Issue。
未测性能，未运行本轮VoiceOver/物理IME，也不由本次证据推断全组件或整页通过。
