# UI Gallery — macOS 图标资源包

保留已确认的蓝紫色玻璃卡片设计。全部图标由同一原图直接缩小导出，保留透明通道；未重新绘制或拆分图层。

## 文件

- `Gallery.icns`：含 10 个标准尺寸/倍率表示的 macOS 图标文件。
- `AppIcon.appiconset/`：Xcode 图标资源，含 10 张 PNG 和 `Contents.json`。
- `Gallery.iconset/`：标准 iconset，含 10 张 PNG，可在 Mac 上用 iconutil 重新编译。
- `Gallery-1024.png`：1024 × 1024 PNG 主图。
- `Source/Gallery-original.png`：1254 × 1254 原始生成图，未经修改。
- `README.md`：本说明。

## 尺寸

以下为传统 macOS asset catalog/iconset 的完整尺寸组合，不是所有项目都必须手工提供 10 张图。

| 逻辑尺寸（pt） | @1x 像素尺寸 | @2x 像素尺寸 |
| --- | --- | --- |
| 16 × 16 | 16 × 16 | 32 × 32 |
| 32 × 32 | 32 × 32 | 64 × 64 |
| 128 × 128 | 128 × 128 | 256 × 256 |
| 256 × 256 | 256 × 256 | 512 × 512 |
| 512 × 512 | 512 × 512 | 1024 × 1024 |

共 10 个尺寸/倍率条目、7 种不同的实际像素尺寸。`Contents.json` 中的 size 是 pt，scale 是倍率。

## Xcode 项目

1. 将 `AppIcon.appiconset` 加入项目的 `Assets.xcassets`。已有同名图标集时先备份，再替换，避免重复。
2. 在 Target 的 App Icons 设置中选中 `AppIcon`。对应 Build Setting 为 `ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon`。
3. 构建后在实际 macOS 环境检查 Finder、Dock、浅色与深色背景，以及不同缩放大小。

这是传统静态 asset catalog 图标包，不是 Icon Composer 的多图层 `.icon` 工程。若项目已经采用 Icon Composer，不要未经调整就同时配置两个同名图标来源。

## 非 Xcode / Rust / 自定义 .app 打包

使用项目打包工具提供的 icon 配置，指向 `Gallery.icns`。具体配置项取决于所用打包工具，不应直接套用其他框架的配置。

手工组织 .app bundle 时，将图标复制为 `YourApp.app/Contents/Resources/Gallery.icns`，并在 `Contents/Info.plist` 指定：

```xml
<key>CFBundleIconFile</key>
<string>Gallery.icns</string>
```

应在构建/打包阶段配置图标并重新签名，不直接修改已签名的发布应用。

## 在 Mac 上重新生成 ICNS（可选）

在本目录执行：

```sh
iconutil -c icns Gallery.iconset -o Gallery.icns
```

## 导出与验证

- 逐个验证了 PNG 尺寸、RGBA 模式及图像可解码性。
- 验证了 asset catalog 的 10 个文件引用及倍率对应关系。
- 验证了 ICNS 文件长度、10 个图像块，并用 Pillow 逐一回读、比对像素。
- 验证了 ZIP 完整性。
- 本环境未运行 Xcode、iconutil 或 macOS 原生图标显示测试；最终效果请在目标 Mac 上确认。
- 小尺寸是高质量缩小版本，尚未进行逐尺寸的手工光学校正。

## Apple 参考文档

- 传统 iconset 尺寸、命名与 iconutil（历史文档，仅用于此格式）：
  https://developer.apple.com/library/archive/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/Optimizing/Optimizing.html
- Asset catalog 图标配置：
  https://developer.apple.com/documentation/xcode/configuring-your-app-icon
- Asset catalog 格式：
  https://developer.apple.com/library/archive/documentation/Xcode/Reference/xcode_ref-Asset_Catalog_Format/AppIconType.html
- 新版 Icon Composer：
  https://developer.apple.com/icon-composer/
