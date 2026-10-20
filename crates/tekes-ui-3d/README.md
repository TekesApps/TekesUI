# TekesUI3D

独立 macOS 模型查看应用，消费 `tekes-ui-scene3d` 的公共 API。启动默认从实际 `rubiks-cube.glb` 文件导入模型；窗口、相机交互和显示时钟由此宿主管理，组件仍不依赖平台事件。GPU 蒙皮、morph、索引网格、法线计算和 GPU 纹理直接合成沿用共享 wgpu 路径。

```sh
cargo run -p tekes-ui-3d --locked
cargo run -p tekes-ui-3d --locked -- /absolute/model.glb
# 也支持 glTF + 相邻 bin / 图片
cargo run -p tekes-ui-3d --locked -- /absolute/model.gltf
# 构建原生应用（默认 release 优化，资源复制到 bundle）
crates/tekes-ui-3d/scripts/build-app.sh
open target/TekesUI3D.app
# 仅最终截图读回 CPU；交互渲染没有读回
cargo run -p tekes-ui-3d --locked -- --snapshot /tmp/tekesui3d.png
```

| 操作 | 效果 |
| --- | --- |
| 左键拖动 / 方向键 | 水平、垂直轨道视角 |
| 右键拖动 / Shift + 左键拖动 / Shift + 方向键 | 独立旋转整个模型 |
| 滚轮 / `+`、`−` | 缩放 |
| `R` | 恢复初始视角、模型朝向和动画时间 |
| Space | 播放 / 暂停第一个动画（魔方顶层转动） |
| `A` | 开关整个模型自动旋转 |
| View / Animation 菜单 | 同上常用操作 |

默认静止，失焦和系统 Reduced Motion 时冻结自动动画，恢复不补播。当前以启动参数导入其他资产；尚无文件选择器。Windows 窗口宿主以后接入，组件的图形接口保持 wgpu。

`assets/generate_cube.py` 是本仓库原创 MIT 资产生成器，不依赖第三方模型或 Python 包。运行 `python3 crates/tekes-ui-3d/assets/generate_cube.py` 可确定性重建 GLB：26 个独立倒角块体、54 个圆角色贴，索引三角网格、顶点颜色、显式硬边法线；9 个顶层块体挂在同一节点，glTF quaternion 动画执行四分之一圈转动和回转。此示例演示模型动画，不含魔方求解或逐层交互逻辑。

验证：`cargo test -p tekes-ui-3d --locked` 包含真实 wgpu 图像测试，覆盖实际 GLB 导入、缩放、两轴视角、模型旋转、复位和动画。仅 PNG 截图路径读回像素；这些测试不代表大模型性能或 Blender 导出验收。

旋转时复用静态文字场景和 GPU 变形结果；菜单勾选只在状态变化时更新。拖动事件合并到显示回调，同一刷新周期不因输入和动画重复绘制；静止时停止显示链接。`TEKESUI3D_PROFILE=debug crates/tekes-ui-3d/scripts/build-app.sh` 可构建调试版，默认 release 用于观察运行性能，不能将 debug / release 的 CPU 数字直接作为同构建优化比。

可重复启动自动旋转：`cargo run -p tekes-ui-3d --release --locked -- --auto-rotate`；加 `--play` 同时播放资产首个 clip。2026-10-20 本机一次原生 Release 测量：默认魔方、960×720 pt、自动旋转、8 秒进程 CPU 时间差 / 墙钟时间约 5.7%（100% 代表占满一个逻辑核）。这是短时运行观测，不是跨设备或固定帧率性能承诺，也不是与旧 debug 构建的同配置优化比。

构建脚本先复制到临时可执行文件，再原子替换并对本地 app 进行 ad-hoc 签名，避免直接覆盖运行中的签名代码页；`codesign --verify --verbose=2 target/TekesUI3D.app` 可检查打包结果。

## Manhattan Skyline

```sh
cargo run -p tekes-ui-3d --release --locked -- --demo manhattan
# 独立 Manhattan 应用，双击直接进入城市
TEKESUI3D_DEMO=manhattan crates/tekes-ui-3d/scripts/build-app.sh
open "target/TekesUI Manhattan.app"
# 交互窗口的 Demo 菜单可随时切换魔方 / Manhattan Skyline
open -n target/TekesUI3D.app --args --demo manhattan
# 可选：自动旋转；失焦 / Reduced Motion 时停止
cargo run -p tekes-ui-3d --release --locked -- --demo manhattan --auto-rotate
```

`assets/manhattan.glb` 是原创 MIT、自包含 glTF 2.0 资产，可由 Blender 导入。用 `python3 crates/tekes-ui-3d/assets/generate_manhattan.py` 可确定性生成。画面参考《模拟城市 4》的城市俯视表现：108 栋建筑，包含受世贸中心、帝国大厦、克莱斯勒大厦轮廓启发的高楼、公园、街道、码头、小船和绿化。建筑位置、比例、岛形均为艺术化布置，未使用真实地理数据，不能作为地图或测绘模型，也未使用游戏资产。

约 4.5 MiB GLB、86,844 顶点、43,422 三角形，合批为两种材质的索引几何；三种原创幕墙 / 石材立面贴图嵌在 GLB 中，不再用随机大色块拼窗。街道与广场带有资产生成阶段烘焙的建筑阴影（不随运行时灯光变化），包含树木、屋顶设备和路面标线。

城市默认使用正交俯视相机和 4× MSAA。MSAA 在 GPU 上解析到共享纹理，不读回 CPU；可继续环绕、缩放、旋转。抗锯齿比单采样多占显存：RGBA8 色彩 + Depth32Float 的 4× 目标约 32 bytes/px，另有共享的单采样输出 4 bytes/px；魔方及 Gallery 原有默认渲染不启用它。纹理 / 几何仅导入时上传，旋转帧只更新相机 uniform。当前没有实时阴影、PBR 反射或波浪模拟。城市没有资产动画，Space 保持暂停；A 可旋转整个场景。

GPU 回归覆盖城市导入、缩放、视角、旋转、复位、120 次旋转不产生 pose 上传 / 变形计算、MSAA 目标 resize，以及两个 Demo 来回切换时 ImageId 和注册图像数量保持稳定。离屏图像与原生窗口检查分别进行，不以它们代表大规模城市 / 持续帧率性能验收。
