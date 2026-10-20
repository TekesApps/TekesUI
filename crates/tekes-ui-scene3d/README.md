# Scene3D（实验组件）

独立 Rust crate：`Asset3D` 加载资产，`Scene3D` 管理播放状态，`Renderer3D` 生成预览；不创建窗口、计时器或线程，也不依赖 Gallery / ClientKit / Kernel。

首批格式为 **glTF 2.0** 的 `.glb` 与 `.gltf`。推荐 Blender 导出 `.glb`，一个文件包含模型、贴图和动画；`.gltf` 也支持邻接 `.bin` / 图片和 data URI。`.blend` 是创作工程，不直接载入。FBX / USD / OBJ 暂不提供导入器。

图形 API 由 **wgpu** 抽象。组件不按 macOS / Windows 分支，不直接管理 Metal 或 DirectX。当前工作区的 wgpu 构建启用了 Metal；后续平台由宿主启用对应 wgpu feature，组件接口与资产模型保持一致。

## 已实现范围

- 默认场景、节点层级、三角网格、索引、顶点颜色、基础颜色贴图（8-bit、UV0）。
- 节点 TRS、骨骼蒙皮（4 个权重）、morph position / normal；动画 STEP / LINEAR / CUBICSPLINE，旋转 LINEAR 使用 quaternion slerp。
- clip 选择、播放/暂停、seek、正向倍速、单次/循环；隐藏与 Reduced Motion 冻结时钟，恢复不补播隐藏时间。外部显示回调调用 `advance(dt)`，`needs_frame()` 决定是否继续请求帧。
- 深度遮挡、透明背景、OPAQUE / MASK；固定预览光照、轨道相机 yaw / pitch / zoom、绕模型中心的 quaternion 旋转；尺寸校验、GPU 资源 resize、明确错误。
- `paint` 把宿主持有的 `ImageId` 放入普通 UI Scene，沿用 UI 的布局、裁剪和叠放规则。

当前材质是**双面漫反射预览**，不是完整 glTF PBR：metallic / roughness、normal / emissive / occlusion、灯光和文件内相机未应用。贴图采用重复寻址、GPU 生成的 mipmap、三线性与各向异性采样；未应用资产自带 sampler、texture transforms。BLEND、非三角网格、超过四个骨骼权重、非 8-bit 图片和 required 扩展明确报错；不支持 Draco / Meshopt / KTX2。optional 扩展忽略并采用核心字段。动画按资产 clip 独立播放，不做 clip 混合、IK 或物理模拟。

这是本地资产预览 API：`.gltf` 的相对路径按源文件目录解析；宿主负责文件授权和资产来源。加载是同步的，宿主应在工作线程载入资产；GPU / UI 合成按宿主的渲染线程约定使用。

## 嵌入

```rust,no_run
use tekes_ui_scene3d::{Asset3D, Scene3D, Renderer3D};
# fn embed(ui: &mut tekes_ui_render::Renderer) -> Result<(), Box<dyn std::error::Error>> {
let mut view = Scene3D::new(Asset3D::load("character.glb")?);
view.select_clip(0)?;
view.play();
let mut gpu = Renderer3D::new(&ui.gpu_context(), &view, (512, 512))?;
gpu.render(&view, 0.0, 1.0)?;
let image_id = ui.register_gpu_image(gpu.output())?;
// 后续帧：advance → render → paint → 宿主 UI render。
view.advance(1.0 / 60.0)?;
gpu.render(&view, 0.0, 1.0)?;
// view.paint(&mut scene, bounds, image_id);
// 缩放时替换 backing storage，ImageId 不变。
gpu.resize((1024, 1024))?;
gpu.render(&view, 0.0, 1.0)?;
ui.replace_gpu_image(image_id, gpu.output())?;
// 退出时 ui.unregister_image(image_id)。
# Ok(()) }
```

3D 与 UI 使用同一 `GpuContext` 的 wgpu device / queue。每帧 compute pass 先做 morph / skin，3D pass 直接读取 GPU 变形结果，再由 UI pass 采样共享纹理；正常路径没有像素读回、map、poll、CPU 等待或整张图像重新上传。两次提交由同一队列维持先后依赖，不跨设备共享资源。

`GpuContext::create_image` 创建 GPU-only RGBA8 sRGB target。3D 在线性空间着色，target 写入时编码到 sRGB；UI 通过 `Rgba8Unorm` view 采样已编码通道，符合现有 UI 的 gamma-space 合成规则。图像接口要求预乘 alpha；当前 OPAQUE / MASK 输出 alpha 0 或 1。普通 CPU 图像 API 保留，GPU 图像拒绝 `update_image`，其他 renderer 的纹理也在创建绑定前被拒绝。

图像首次登记后保持同一 ImageId；重绘不重新登记。viewport resize 重建 color / depth target，用 `replace_gpu_image` 原位替换绑定。Producer 或 image handle 释放后，登记表仍持有有效纹理；注销只释放本身引用，已提交的工作由 wgpu 保持资源存活。

网格基准顶点和全部 morph position / normal delta 在初始化时一次上传；每帧 GPU compute 先叠加 morph，再执行四权重蒙皮与法线逆转置变换。需要变形时，compute 对每个原始顶点只计算一次；顶点、morph 和输出 buffer 不按三角形展开。初始化后没有逐顶点 CPU 计算或顶点数据上传。

有 authored normal 的网格保留法线逆转置变换，并通过 Uint32 index buffer 执行 `draw_indexed`，复用顶点着色结果。缺失 authored normal 时，顶点着色器按原始索引读取 GPU 变形后的位置，以 `cross(p1-p0, p2-p0)` 计算当前三角形的 flat normal；这一分支使用 indexed vertex pulling，只重复着色器调用，不创建展开后的顶点、morph 或逐面法线缓存。不依赖 primitive_index 扩展，也不使用屏幕导数近似。原始 glTF 的 UV / hard-normal seam 顶点保持独立，不按位置焊接。

CPU 仍负责关键帧插值、节点层级求值、joint world × inverse bind 矩阵与 morph 权重采样；每帧只上传这些紧凑 pose 数据（以及相机），上传量随节点/关节/morph 数变化，不随顶点数变化。加载时的初始取景仍使用一次 CPU 参考几何计算。

诊断接口 `geometry_upload_bytes()` 记录构造时基准顶点、morph 和索引的上传字节数；`geometry_resident_bytes()` 统计这些缓冲加 GPU 变形输出的请求大小（不包含材质、pose、viewport target 或驱动开销）。`last_frame_pose_upload_bytes()` 记录最近一帧的矩阵/权重上传量（不含相机的 128 字节）。GPU 测试将原始顶点及索引扩大 4096 倍，pose 上传上限仍为每帧 216 字节；这是单网格、两关节、两 morph 的夹具结果，不是通用模型的固定开销。静态输入 buffer 没有 COPY_DST，GPU 变形输出也没有 COPY_DST；生产输出没有 COPY_SRC，测试才额外启用几何读回。

几何缓冲由原来的 `(160 + 32 × morph数) × 索引数` 字节，改为 `(160 + 32 × morph数) × 原始顶点数 + 4 × 索引数`（无 morph 时另有 32 字节最小占位）。四顶点、十二索引、两个 morph 的夹具由 2688 降至 944 字节，约减少 65%；将同组索引重复 4096 次时由 11,010,048 降至 197,504 字节。后者是高复用压力夹具，不代表一般模型的压缩率。没有顶点复用的源网格仍需增加索引缓冲；目前索引统一为 Uint32。

默认设备需要至少五个 compute storage bindings，超出 storage / dispatch 上限的资产返回明确错误，不静默退回 CPU 蒙皮。未完成大模型或 60/120Hz 性能验收。自动取景基于初始 pose；极端动画位移可能出画。宿主可用 `Camera3D` 调整两轴视点、缩放与模型朝向。

Gallery：`TEKESUI_GALLERY_PAGE=component-scene3d/playground cargo run -p tekes-ui-gallery`。侧栏 `3D → Rubik’s Cube`，默认 MIT 本地魔方 GLB（约 170 KiB），无网络素材；拖动视角、右键 / Shift 拖动模型、滚轮缩放，点击预览后支持键盘与动画控制；可通过 `TEKESUI_SCENE3D_ASSET=/absolute/model.glb` 指定本地文件，需重新启动 Gallery。预览自动播放首个 clip，公共 API 可选择其他 clip。该页属于独立 crate 的 example registry；controls 公共类型审计仍只覆盖 `tekes_ui_controls`。

验证：

```sh
cargo test -p tekes-ui-scene3d --locked
cargo test -p tekes-ui-scene3d --locked -- --ignored # 实际 wgpu adapter，失败不跳过
cargo test -p tekes-ui-render --test gpu_images --locked -- --ignored
cargo run -p tekes-ui-scene3d --example snapshot --locked
```

`snapshot` 是第二个公开 API 消费者，覆盖资产 → 同设备 wgpu target → UI image → PNG，只有最终截图读取 CPU 像素。GPU 几何对照测试覆盖 morph/skin 组合、四权重混合、joint 层级、inverse bind、非均匀/负缩放、奇异矩阵和 seam 顶点；经索引重建后的位置、UV、颜色和 authored normals 与 CPU 参考几何按 5e-5 绝对容差比较。flat normal 另通过方向编码的 GPU 图像与展开的 CPU 参考实现对照，检查面内法线、朝向、硬边和 UV seam；面内颜色容差 2/255，仅允许少量共享边像素因位置浮点误差落到相邻面。数值、GPU 和原生交互属于不同证据；GPU 通过不代表 Blender 实际导出资产、原生交互、VoiceOver、完整材质或性能已验收。

参考：[Khronos glTF](https://www.khronos.org/gltf/)、[Blender glTF 手册](https://docs.blender.org/manual/en/latest/addons/scene_gltf2.html)、[wgpu 30 后端](https://docs.rs/wgpu/30.0.0/wgpu/)。

## 相机与独立示例

`Camera3D` 提供 `orbit(yaw_delta, pitch_delta)`、`zoom_by(distance_factor)`、`rotate_model(yaw, pitch, roll)` 和 `reset()`；角度以弧度表示，缩放系数小于 1 放大。输入校验有限值，pitch 限制在极点以内，zoom 限制在 0.2–8。模型旋转绕初始拟合中心进行，与相机视角独立；GPU 同步旋转位置与法线，固定世界光照随表面朝向更新。相机变化只更新 128 字节 uniform，不重新上传顶点或 morph 数据。

```rust,no_run
# fn update(gpu: &mut tekes_ui_scene3d::Renderer3D, view: &tekes_ui_scene3d::Scene3D) -> Result<(), tekes_ui_scene3d::Error> {
let mut camera = tekes_ui_scene3d::Camera3D::default();
camera.orbit(0.6, 0.2)?;
camera.zoom_by(0.85)?;
camera.rotate_model(0.2, 0.0, 0.0)?;
gpu.render_camera(view, &camera)?;
# Ok(()) }
```

独立宿主见 [`tekes-ui-3d`](../tekes-ui-3d/README.md)，展示真实魔方 GLB 导入、拖动、滚轮、键盘与原生菜单。原有 `render(view, yaw, zoom)` 保留。

旋转优化：以 clip / time 缓存已采样姿态；只有相机或整个模型朝向变化时，pose 上传与 compute dispatch 均为零。动画时间变化时再采样，并逐网格比较 model / joint palette / morph weights，只上传、变形发生变化的网格。`last_frame_deformed_meshes()` 提供最近帧的 compute 网格数。魔方初次执行 26 个网格；顶层转动只执行 9 个网格、上传 720 字节；停留关键帧期间变化为零。

深度采用 `Depth32Float` + infinite reversed-Z（near→1、远处→0、清零、Greater 比较），避免缩小时紧贴的色贴和底面因传统深度映射损失精度而出现斜向黑纹。GPU 回归对多个距离及倾斜角，将紧贴表面和独立前景表面逐像素比较，排除内部穿插。

Gallery 的预览 target 现按实际图像区域 × backing scale 自适应（上限为设备限制与 4096 的较小值），resize 保持 ImageId 和几何 buffer；暂停 / Reduced Motion 下也完成尺寸收敛。持续原生旋转测试可用 `TEKES_PERF=scroll:60 TEKES_PERF_CONTENT=scene3d TEKES_PERF_GPU_TIMING=0 cargo run -p tekes-ui-gallery --release --locked`，增加 `TEKES_PERF_SCENE3D_ANIMATION=1` 同时播放 clip。实际前台 120Hz / Retina 测量及原始数据见 [测试记录](../../docs/design/scene3d-adaptive-20261020.md)。

建筑 / 城市消费者可调用 `Camera3D::set_orthographic(true)` 使用正交投影，仍沿用缩放和反向 Z 深度；默认保持透视。`Renderer3D::new_antialiased` 提供可选的 4× MSAA，在 GPU 上 resolve 到共享输出图像；默认 `new` 仍为单采样，Gallery 现有显存预算不变。抗锯齿渲染器 resize 时重建匹配采样数的色彩和深度附件。

### Generic GPU instances and environment

`configure_instances` accepts named glTF-node batches of `Instance3D` and closed `Route3D` polylines. Immutable route/instance buffers upload once; omitted nodes retain one draw, empty batches hide a node. OPAQUE/MASK instancing is supported; BLEND instances are rejected until per-instance sorting is implemented. Existing single-instance consumers keep identity bindings.

`set_environment` updates host-owned time/wind (16 bytes, only when changed). Vertex shaders evaluate route arc lengths, bounded speed variation/lateral offsets, rooted wind, traveling water waves, pivot swing and finite floor-by-floor `BuildUp` reveal. Wind/wave normals use the analytic inverse Jacobian; scale and rigid rotation correctly transform normals. Geometry, skin and morph buffer reuse is preserved. BuildUp clips the prebuilt structure without stretching floors; it does not create a watertight cap on the moving cut plane.

Base-color textures generate their mip chain once on GPU in linear light. Trilinear + 8× anisotropic sampling reduces minification shimmer; mip chains add roughly one third of base texture storage. No per-frame texture upload or CPU downsampling is involved.


### Reuse draw commands and share the UI frame

Opaque/MASK draw commands are cached in a wgpu RenderBundle, including indexed and flat-normal paths. Uniform writes and GPU skin/morph/analytic motion remain live. Instance reconfiguration invalidates the bundle; resize only replaces attachment views. Camera matrices upload only when changed.

For native GPU composition, prefer `Renderer::render_with_gpu` plus `Renderer3D::encode_camera`. Acquisition happens first, component passes precede UI sampling, and the frame uses one queue submission. `render_camera` remains available for standalone submission. The UI's `render_to_rgba8_with_gpu` supplies the same ordering for image tests and snapshots. See `docs/design/gpu-component-performance.md` for lifecycle rules, measurement and reusable regression checks.

`Deformation3D::StagedReveal` generalizes the grow/hold/remove lifecycle: fixed-size steps reveal from the base, hold after completion, then disappear from the top down. Only GPU time changes; instance storage stays 112 bytes and the finite lifecycle requires no per-frame instance upload.
