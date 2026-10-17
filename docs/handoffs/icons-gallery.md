# Icons Gallery 独立交付

本页为本机旧资产只读预览，不表示旧图标已完成原创性、MIT 许可或公开导入核对。当前批准范围是旧集 133 项与 Gallery 基础 10 项，共 143 项；5 个已作废原稿继续排除，不扩大范围。

## 文件和接口

- `crates/tekes-ui-gallery/src/gallery/icon_catalog.rs`：目录读取、排除清单、SVG 预算、光栅化与 GPU 缓存。
- `crates/tekes-ui-gallery/src/gallery/pages/icons.rs`：两组统一搜索、计数、固定三列布局、可见项判定及页面绘制。
- 此文档及 `target/icons-review-20261015/`：独立验收说明与本机渲染证据。后者含私有图标画面，留在忽略的本机输出目录，不提交或公开。

现有 `extent`、`visible_legacy`、`build_catalog`、`prepare` 调用接口未变。新增 `preview_sizes()` 仅供本模块与页面共用 JSON 的 `size-icon-small`、`size-icon-standard`、`size-icon-preview`；当前为 12 / 16 / 24 pt。`fixture()` 仅在测试构建可用。Gallery/Host/Viewport/Nav/Text 的事件与滚动归属由主聊天整合，本次不改这些共享文件。

## 数据和绘制约定

显式设置 `TEKESUI_ICON_DIR` 可获得可复现的来源选择。自动查找同级 Tekes 源目录或 `~/dev/Tekes/Tekes/Assets.xcassets/TekesIcons` 仅为本机开发便利，不能作为自包含交付。复现还应比对本机验收清单中的 133 个母版 SHA256；未来只有经过原创性与许可确认后才能另行正式导入。

- 支持 imageset 矢量母版和平铺 SVG；imageset 的 2x/3x 不重复列项；平铺导出有母版时也忽略 `-2x` / `-3x` 副本。同名 imageset/平铺项只显示一次。
- 文件实际读取上限 256 KiB，超过则拒绝；解析前限制至多 8192 个 `<` 标记。文件打不开、编码不合法、SVG 畸形或超预算均记录 notices。已作废名称先排除，其他含 C2PA 标记的输入拒绝。
- 这些是本地预览预算，不是通用不可信 SVG 沙箱。禁用字体及外部图像解析，不加载网络/文件引用的图像。
- 搜索是名称的字面包含匹配，不区分大小写。`%query%` 表达包含语义，用户只输入 query；百分号、下划线不作为通配符。两组原名保留，不把相似语义图标合并。
- 页面范围按筛选结果推导；绘制和可见项加载使用同一份格子几何。最后一行包含在文档 extent 内，屏外格子不绘制。
- 可见旧图标才进行光栅化，每项三种逻辑尺寸。GPU 缓存按主题和倍率区分，切换前 unregister 旧图像，滚动/搜索复用当前缓存。当前来源最多 133×3 个句柄，遍历整个列表后保留以便回滚；随 renderer 销毁回收。不做后台持续动画。
- SVG 使用 `PrimaryText` 的原始 sRGB 通道及预乘 alpha，沿用渲染器 sRGB 编码空间混合契约，不额外 `to_linear()`。边框、文字角色和颜色来自现有 token；格子大小是 Gallery 固定夹具几何，不代表新增生产控件规范。
- 无效倍率（非有限、非正或大于 8）不进行 GPU 分配；不会替换已有有效缓存。

## 验证

普通独立测试：

```sh
cargo test --locked --offline -p tekes-ui-gallery icon_catalog
cargo test --locked --offline -p tekes-ui-gallery gallery::pages::icons
cargo clippy --locked --offline -p tekes-ui-gallery --all-targets -- -D warnings
```

本机完整集验收需明确指定私有目录，默认忽略，不让 CI 或固定 Gallery 快照依赖私有资产：

```sh
TEKESUI_ICON_DIR='<本机已批准旧图标目录>' \
TEKESUI_ICONS_REVIEW_OUT=target/icons-review-20261015 \
cargo test --locked --offline -p tekes-ui-gallery \
  local_full_catalogue_pixels_and_scroll_culling -- --ignored
```

本轮结果：

- 通过：8 个常规独立测试，覆盖来源缺失、畸形/编码/体积/标记预算、作废排除、两种目录结构去重、非法倍率、主题/倍率缓存复用与释放、sRGB 像素颜色、字面包含匹配和统一筛选末行几何。
- 通过：显式本机完整集测试，读取 133 项且 notices 为空；两组共 143 项，`SeArCh` 返回 3 项；逐行遍历使全部 133×3 张纹理成功登记，主题/倍率切换后句柄数量仍为 399，没有逐轮累加。
- 通过：四主题 × 1x/2x × 顶部/底部/搜索/空结果，共 32 张离屏 PNG。搜索框复用实际 TextDemo 搜索 surface，仅脚本设置输入状态；不是原生窗口截图。页面区域使用与真实页相同的裁剪边界。
- 通过：严格 Clippy 与本次两个 Rust 文件格式检查。
- 已目检：深色 1x 顶部、浅色 2x 末行、深色增强对比 2x 搜索、浅色增强对比 1x 空结果；文字槽、边框和图标三尺寸未见截断或错位。其余变体已生成，未逐图声称人工验收。
- 待主聊天统一验收：真实窗口的手势 owner、滚动条捕获、resize 后位置、真实系统 IME、焦点/剪贴板、120 Hz 性能及空闲显示回调归零。离屏几何与 GPU 测试不能替代这些验收。

固定公共快照继续使用 10 项基础图标的确定性夹具，不隐式加载私有 SVG。完整集的本机源文件及 32 张证据 SHA256 保存在 `target/icons-review-20261015/verification.json`；资产未被复制入源码或修改。


## 2026-10-16：提案接入与新旧对比

51 个提案随 Gallery 编译嵌入：38 个新增、5 个重绘、8 个优化。133 个有效旧图标中与优化稿同名的 8 项保留为 before 变体，不覆盖私有源；页面合计 186 个逻辑条目（10 基础 + 133 原有 + 43 新名字）。优化条目优先显示，每张卡左旧右新，各列 12/16/24 pt，全部共享名称的大小写无关包含搜索，另支持中文标签。

变更仅在 icon_catalog.rs、proposal_assets.rs、pages/icons.rs 与本文。固定公开快照仍使用基础夹具。内嵌资产只引用 design/proposals/icons-20261015/svg，旧图标继续从外部只读目录加载。缺少旧源时提案仍可用，优化卡提示旧稿未加载。

验证：常规图标测试、提案合并/唯一性/缺源/双变体 GPU 缓存、跨新旧提案搜索测试通过；显式完整集逐行光栅检查通过（176 个条目 + 8 份旧稿，552 张尺寸纹理），四主题 × 两倍率 × 四状态共 32 张离屏图。真实原生窗口确认 186 项、CAD- 搜索 14 项、SETT 搜索 1 项且旧稿优化稿并排。已清空搜索并保留原生窗口供查看。

独立窗口包：target/icons-proposals-preview/TekesUI Icon Proposals.app。原生截图：target/icons-proposals-review/native-icons-comparison.png。离屏图位于 crates/tekes-ui-gallery/target/icons-proposals-review。没有更新三个消费应用的图标。

补充边界：本轮严格 Clippy 重跑停在 controls 的既有 build script，已终止本任务的等待进程，未得出通过结论；前述 Gallery 编译、11 个普通图标测试（分两次）、完整集测试和原生窗口验证已有成功结果。


## 2026-10-16：8 个优化稿获批为默认

用户明确批准 8 个优化稿作为默认。正式真源移至 crates/tekes-ui-icons/assets，通过 Icon::named / Icon::svg 复用，不再从提案目录取得默认稿。Gallery 的 8 条提案引用改为这个共享 crate，卡片标明“默认使用”，旧稿 before 仍从外部只读加载并排显示，条目总数仍为 186。其余 43 个提案状态不变。没有修改三个 Swift 应用的资产、映射或发布状态。

本轮验证：tekes-ui-icons 单测及严格 Clippy 通过，Cargo 打包清单包含全部 8 个默认 SVG，逐字节确认与获批稿一致。Gallery 增加共享默认来源及 before 保留测试；重跑当前受并行动效 pages/motion.rs 的 RecipeKind 非穷尽分支错误阻挡，未声称新窗口已验证。
