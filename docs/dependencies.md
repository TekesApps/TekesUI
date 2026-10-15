# 外部依赖复核

复核日期：2026-10-15。数据来自 crates.io API，日期为 crates.io 的发布时间（UTC）。charter 与 roadmap 里写的版本是 2026-10 调研时的状态，本文是开工前的复核结果。以后升级依赖时在本文末尾追加记录。

## 结论

- 列出的 22 个 crate 许可证都与 MIT 兼容，没有 GPL、LGPL、MPL。
- charter 的假设全部与现状一致：wgpu 30、objc2 0.6、objc2-app-kit / objc2-core-text 0.3、AccessKit 2026-09 版本、Parley 0.12、winit 0.31 beta。
- 工具链固定为 Rust 1.95.0（见 `rust-toolchain.toml`），满足下表全部 MSRV，最高要求是 1.88。
- 要注意的一处：accesskit_macos 0.27.1 仍依赖 objc2 0.5 和 objc2-app-kit 0.2，见下文。

## 版本表

| crate | 最新稳定版 | 发布日期 | 更新的预发布 | 许可证 | MSRV | 与 MIT 兼容 |
|---|---|---|---|---|---|---|
| wgpu | 30.0.1 | 2026-08-22 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| wgpu-hal | 30.0.1 | 2026-08-22 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| wgpu-types | 30.0.1 | 2026-08-22 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| naga | 30.0.1 | 2026-08-22 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| objc2 | 0.6.5 | 2026-10-05 | 无 | MIT | 1.71 | 是 |
| objc2-foundation | 0.3.2 | 2025-10-04 | 无 | MIT | 1.71 | 是 |
| objc2-app-kit | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| objc2-quartz-core | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| objc2-core-foundation | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| objc2-core-graphics | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| objc2-core-text | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| objc2-metal | 0.3.2 | 2025-10-04 | 无 | Zlib OR Apache-2.0 OR MIT | 1.71 | 是 |
| block2 | 0.6.2 | 2025-10-04 | 无 | MIT | 1.71 | 是 |
| raw-window-handle | 0.6.2 | 2024-05-17 | 无 | MIT OR Apache-2.0 OR Zlib | 1.64 | 是 |
| accesskit | 0.25.1 | 2026-09-25 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| accesskit_macos | 0.27.1 | 2026-09-25 | 无 | MIT OR Apache-2.0 | 1.87 | 是 |
| parley | 0.12.0 | 2026-10-09 | 无 | Apache-2.0 OR MIT | 1.88 | 是 |
| winit | 0.30.13 | 2026-03-02 | 0.31.0-beta.3（2026-09-04） | Apache-2.0 | 1.70（beta 1.86） | 是 |
| bytemuck | 1.25.2 | 2026-07-19 | 无 | Zlib OR Apache-2.0 OR MIT | 未发布 | 是 |
| pollster | 1.0.1 | 2026-07-10 | 无 | Apache-2.0 / MIT | 1.69 | 是 |
| taffy | 0.15.0 | 2026-10-12 | 无 | MIT | 1.71 | 是 |
| icu_segmenter | 2.3.0 | 2026-08-13 | 无 | Unicode-3.0 | 1.88 | 是 |

winit 只列作参照，按 charter 不用于 mac 平台层。

## 与 charter 的对照

| charter 写的 | 现状 | 判断 |
|---|---|---|
| wgpu 30 | 30.0.1 | 一致 |
| objc2 0.6 | 0.6.5；0.7 未发布 | 一致 |
| objc2-app-kit / objc2-core-text 0.3 | 0.3.2 | 一致 |
| AccessKit 2026-09 版本 | accesskit 0.25.1、accesskit_macos 0.27.1，都在 2026-09-25 发布 | 一致；依赖问题见下 |
| Parley 0.12 | 0.12.0 | 一致 |
| winit 0.31 beta | 0.31.0-beta.3，尚无稳定版 | 一致 |

taffy 0.15.0 在 2026-10-12 刚发布，charter 没有写版本。F2 引入时再复核。

## wgpu 渲染到 CAMetalLayer 的入口

wgpu 30.0.1 支持直接从 CAMetalLayer 指针建 surface：

```rust
unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr)) }
```

这个变体带 `#[cfg(metal)]`，metal 是默认 feature，只在 Apple 目标下生效。docs.rs 在 Linux 上构建，页面里看不到这个变体，要以源码为准（`src/api/surface.rs`）。

## objc2 版本是否统一

- objc2-app-kit 0.3.2 与 objc2-quartz-core 0.3.2 依赖 `objc2 >=0.6.2, <0.8.0`。
- wgpu-hal 30.0.1 在 Apple 目标上依赖 `objc2 ^0.6.3` 和各个 objc2-* 的 `^0.3.2`。
- 两边会统一到 objc2 0.6.5 与 objc2-* 0.3.2。

accesskit_macos 0.27.1 依赖 `objc2 ^0.5.1`、`objc2-app-kit ^0.2.0`、`objc2-foundation ^0.2.0`。F6 引入它之后，同一个构建里会同时出现 objc2 0.5 和 0.6。不同版本的类型互不相同，可以共存，代价是编译时间和二进制体积。我们自己的代码只用 0.6 / 0.3 的类型，不在 crate 之间传 0.5 的类型。F6 开工时再复核 accesskit_macos 是否已升级。

## 2026-10-15：tekes-ui-text 新增依赖（F1）

数据来自 crates.io（`cargo info`）与 `cargo metadata`。

| crate | 版本 | 许可证 | 用途 | 与 MIT 兼容 |
|---|---|---|---|---|
| objc2-core-text | 0.3.2 | Zlib OR Apache-2.0 OR MIT | CoreText 绑定：排版、字体回退、光栅化（只在 macOS 目标编译） | 是 |
| serde | 1.0.229 | MIT OR Apache-2.0 | 布局导出（`tekesui-layout/0`）的序列化 | 是 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 同上，生成与读取 JSON | 是 |

serde 与 serde_json 的传递依赖：serde_core、serde_derive、syn 3.0.9、proc-macro2、quote（均为 MIT OR Apache-2.0），itoa（MIT OR Apache-2.0），memchr（Unlicense OR MIT），zmij（MIT）。没有 GPL、LGPL、MPL。

tekes-ui-text 还用到已在依赖表中的 objc2-core-foundation 与 objc2-core-graphics（0.3.2），只多开了几个 feature（CFString、CFAttributedString、CFDictionary、CFNumber、CFArray、CGBitmapContext、CGContext 等）。
