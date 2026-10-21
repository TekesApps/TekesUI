# #31 独立候选移交

精确开发源码身份：`545ef8d70d89404f11d916f05e417404277507b07dea808aac051339544431a1`。

归档：`label-layout-545ef8d70d89.tar.gz`，SHA-256：`6d00adbd7a975823ad772a2fef22d925e4756f4c28c3dc036115abb5d3313630`。476文件、21路径增量、880570字节。完整文件与逐路径增量见 portable-manifest.json。

此包基于受保护的2b16开发候选，仅增加#31共享标签能力、已有文本宿主适配、公开最小消费者与测试、语义设计契约及其生成产物。5个既有生产文件的整改前副本与2b16逐字节相同，因此没有复制后续集合、Tooltip或其他组件整改。默认控件/文本结构体字面量和自然换行保持；缺失文字宿主能力明确报错。文本导出测试所需Schema与既有公开8cf完全相同；它补齐测试输入，不改变导出格式。第一轮遗漏Schema的失败归档与日志另行保留，不能使用d070候选。

原2b16、bd3、公开8cf及AgentKit已有冻结包均未覆盖。原有设计能力JSON仍明确是 contract-binding fixture，仅供原Rust绑定测试读取契约状态/token字段，不是当前运行时能力/原生/成熟度清单。库不新增Summary/Agent业务UI或状态；继续通过既有业务组件接入。

公共接法：

```rust
Content::new(id, TextContent::new(full_original_title))
    .with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))?
```

同样适用于ButtonContent；CompoundButton在装饰TextContent槽启用，完整动作名称仍由原CompoundButton提供。TextSurfaceHost实现可选shape_label，复用自己的CoreTextShaper/字体注册表和push_visible；不要另造字体系统、编辑器或预截断字符串。真实适配与最小场景见examples/composition-consumer/examples/native.rs与src/label_content.rs。CompositionError新增MissingLabelServices和UnsupportedLabelLayout；外部穷尽match须适配。可编辑文本不使用只读标签几何。

发送方已从这个确切归档在全新临时路径复验：50text、388controls、176独立consumer全部通过；原生示例构建、三个包严格Clippy通过；432核心GPUcases/864frames与独立native CTLine/明确多行文字参考一致。所有476文件与锁在测试后未变，全部本地依赖在解包目录内；复验输出已清理并恢复共享缓存根通道。见portable-checks.json、portable-acceptance.json、sender-summary.json及portable-evidence。

macOS/CoreText/Metal与Rust1.95.0；首次取registry依赖需网络或已缓存。先核对归档和manifest，解包到新目录，在source运行：

```sh
export CARGO_TARGET_DIR="$PWD/target/review-cache"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_DEV_INCREMENTAL=false
export CARGO_PROFILE_TEST_INCREMENTAL=false
export TEKESUI_LABEL_GPU_OUT="$PWD/evidence/label-core"
cargo +1.95.0 test --locked -p tekes-ui-text
cargo +1.95.0 test --locked -p tekes-ui-controls
cargo +1.95.0 test --locked --manifest-path examples/composition-consumer/Cargo.toml
cargo +1.95.0 build --locked --manifest-path examples/composition-consumer/Cargo.toml --example native
cargo +1.95.0 clippy --locked -p tekes-ui-text --all-targets -- -D warnings
cargo +1.95.0 clippy --locked -p tekes-ui-controls --all-targets -- -D warnings
cargo +1.95.0 clippy --locked --manifest-path examples/composition-consumer/Cargo.toml --all-targets -- -D warnings
```

原生示例选择label场景（native label或TEKESUI_COMPONENT_CASE=label）。窗口/打包/系统激活仍由宿主管理；外接屏优先，不测试性能。完成后关闭自己启动的进程，清理自己拥有的构建产物，保留源码/日志/图像。

**证据身份不能混用**：Root全部WIP的实际原生身份是22caa4f3…，在U2790B@2通过26个真实活动检查点，104个组件框与公共API精确一致，覆盖宽窄和三档字号、键盘/坐标指针/AX、禁用/加载/恢复；两个动作各4次，完整AX名称保留。该Root窗口已退出并清理，VoiceOver关闭。证据位于同级工作区的design/reviews/label-content-integration-20261021/native-acceptance.json、native及gold7-label-delta.json。它不是545候选的原生通过证据。本545候选目前仅sender独立构建/API/GPU通过，原生示例编译通过。

请在当前共享修复验证完整后，对545精确源码独立构建、接入现有紧凑标题/内容槽并验收：窄宽窗口、长中英文/emoji/组合字素、三档字体倍率；max-lines=1不增为多行，省略不拆字素/不重叠；完整标题仍为AX name；原按钮hit/AXframe、focus、hover/pressed、disabled/loading仍由库掌管。复验正常自然换行默认路径、CompoundButton，以及既有#22/#26阅读/同步不回退。产品页面按ClientKit/Gold7分别记录。

请按545精确身份返回逐项结论与能否建议提升开发冻结候选；此前冻结包和默认公开pin保持原样。独立原生/真实AgentKit接入、匹配产品视觉、接收方结论、公开不可变commit与CI尚未由发送方此包证明。开发候选不是公开pin；没有提交、推送、tag、merge、关闭Issue或自动冻结提升。
