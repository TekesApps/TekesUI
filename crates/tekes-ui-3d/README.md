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

## AI 健康小屋（带窗版）

```sh
cargo run -p tekes-ui-3d --release --locked -- --demo health-cabin
TEKESUI3D_DEMO=health-cabin crates/tekes-ui-3d/scripts/build-app.sh
open "target/TekesUI Health Cabin.app"
```

根据用户提供的《AI健康小屋（带窗版）》PDF，第 6 页平面图及第 12–14 页立面，建立独立的 `assets/health-cabin.glb`。单位为米，舱体宽 3.905、深 2.635、高 2.886，离地 0.185；含白色圆角外壳、960 mm 中央内开玻璃门、两侧 1273 mm 上翻窗 / 固定玻璃、招牌、活动踏步、脚轮、支架和后置空调。桌椅、AED、储物柜和检测设备依平面位置示意，未标注细节和圆角半径为建模近似，不作为施工放样模型。未嵌入原 PDF、联系方式、周边建筑或渲染照片。

`Space` 播放 / 暂停内开门和上翻窗动画，`R` 复位，`A` 自动旋转；也可从 Demo 菜单切换。采用真实 glTF alpha BLEND 玻璃及 GPU 4× MSAA，室内物体可透过玻璃看见。动画只更新活动节点的紧凑 pose，节点顶点变换仍由 GPU 执行；静止时停止动画调度，旋转时不上传 / 重算模型顶点。

生成器 `assets/generate_health_cabin.py` 需要 Pillow，以及 macOS STHeiti 字体生成中文招牌；模型和招牌图像均嵌在 GLB 中，应用运行不需要 Python、字体文件或原 PDF。它是尺寸参考的交互展示模型，当前不是 PDF 渲染图的 PBR / 光追级复现。

## Living City Block

```sh
cargo +1.95.0 run -p tekes-ui-3d --release --locked -- --demo city-block
TEKESUI3D_DEMO=city-block crates/tekes-ui-3d/scripts/build-app.sh
open "target/TekesUI City Block.app"
# Reproducible construction stage (seconds)
target/release/tekes-ui-3d --demo city-block --time 20 --snapshot /tmp/city-block.png
```

原创滨水街区：四栋已建成建筑、一处逐层施工工地及回转塔吊、10 辆车、28 位行人、18 株树（三种树形与带叶脉的叶片）、口袋公园、池塘和动态滨水水面。人车与树缩小到街区比例；行人各自速度不同，车辆用有界、错相的速度变化保持同车道间距。工地约每 8 秒长一层，完成后停留；`R` 可重新开始。`Space` 暂停 / 播放，拖动环绕、滚轮缩放、`A` 自动旋转，失焦 / Reduced Motion 冻结。

模块边界：`tekes-ui-city` 只保存配置与时钟，`district.rs` 做宿主适配，`tekes-ui-scene3d` 提供通用 GPU 实例、路径、摆动、风、水波和渐进显露；模型生成器与 GLB/JSON 独立。`assets/generate_city_block.py` 需要离线 Pillow，可确定性重建当前约 12 MB 自包含 GLB；运行应用无需 Python。未使用第三方模型或游戏资源。

实例、路径、索引网格和纹理在导入 / 配置时上传。动态帧更新 16 字节环境时间与最多 16 字节实例时钟 uniform；相机矩阵变化时才更新 128 字节相机 uniform；不逐帧上传人物、车辆、树叶或施工顶点。叶片使用 glTF MASK，避免透明实例排序；材质 mipmap 在导入时由 GPU 生成，使用三线性及 8× 各向异性过滤，配合 4× MSAA 缓解缩小时的锯齿与纹理闪烁。mipmap 约增加三分之一纹理显存。当前光照仍为简化漫反射，使用局部灯与粗粒度软阴影代理，不提供 PBR 反射或完整交通模拟。

测试分别隔离车流、行人、树木、水面、施工高度和塔吊，比较真实 GPU 图像；持续 120 帧检查实例 / 几何上传保持为零，并回归暂停、Reduced Motion、复位及 Demo 切换。离屏测试不代表原生窗口持续帧率或城市规模性能验收。


2026-10-20 街区 CPU 优化：通用 Scene3D 缓存不透明/MASK 绘制指令和目标视图，静止相机不重复上传 uniform；独立 3D 宿主通过 UI 的 `render_with_gpu` 把组件与 UI 编入同一帧、一次提交。保持画面与刷新策略；详情、可复用测量脚本和对照证据见 `docs/design/gpu-component-performance.md`。

施工分三个独立节点：主体框架、稍后跟进的外立面、外挂脚手架。脚手架包含钢管、斜撑、踏板与护栏，随楼层逐段搭高；外立面完工后保留约 6 秒，再从顶层向下拆除，楼体保持完整。当前配置约第 49 秒开始拆架、约第 63 秒拆完；`--time 46` / `--time 55` / `--time 80` 可分别查看建成带架、拆架中与拆完的状态。`R` 重新开始，暂停与 Reduced Motion 同时冻结全流程。


City Block also includes a procedural motorboat with a curved GPU wake. Water, boat buoyancy and wake use the same traveling wave field; the river bed stays below the lowest wave trough. Construction scaffolding is dismantled first, then the independently swinging tower crane retires over three seconds (approximately 64.8–67.8 seconds in the bundled recipe). The finished building remains. Pause, reset, inactivity and Reduced Motion use the existing district clock for all these effects.


City Block authoring uses a consistent four metres per scene unit: ordinary stories are 3.2 m, ground stories 3.8 m, and entrance doors 2.24 m. Facade UVs use complete window bays and one row per actual story. People, passenger cars, street trees, benches, sidewalks and river railings use this same scale. The construction base is permanent, while independent hoarding and stored materials retire on the GPU after scaffolding removal. Run `python3 crates/tekes-ui-3d/assets/check_city_block.py` to check exported GLB geometry, atlas story spacing, actor dimensions and normals without Pillow or the runtime. The helper's optional facade sizing leaves existing generators' default UV behavior unchanged.

City Block variety uses eight volumetric pedestrian appearances (adults, children and seniors, with independent skin/hair/clothing colors). `Activity` in the data-only city recipe selects walking, strolling, listening with headphones, or jogging with bent arms and a forward-leaning torso. Routes, individual speeds, bounded speed changes and stride parameters are configured once and evaluated by the existing generic GPU motion. Vehicles include an original streamlined electric sedan, police car, ambulance, fire engine and refuse truck. Roof HVAC layouts and door/awning palettes use seeded offline variation with roof clearances; regeneration is deterministic. The GLB is approximately 4.35 MB; generation and appearance selection add no per-frame CPU geometry work.

The module boundary remains: `scene3d` owns reusable GPU rendering/motion primitives; `city` owns recipe data, activity labels and clock; `3d::district` maps that data to generic instance batches; offline assets own visual geometry and palettes. City-specific activity or vehicle logic does not belong in the renderer. This is a lightweight procedural scene runtime; collision avoidance and pathfinding are not implemented by the current route animation.

The architecture authoring is now split into `assets/city_architecture.py` (original facade atlases, setbacks, window/sill/cornice relief, balconies) and `assets/city_shopfronts.py` (aperture shells, recessed street entrances, displays, signs, canopies and furniture). Upper facades use four-by-four 512×512 atlases; masonry and signs have separate small original textures. Three stepped buildings and the completed construction facade retain the same metric story heights. Entrances explicitly face their assigned street: apartments west, office north, hotel east, shops south, construction west. The office and hotel have separate 3.04 m × 2.4 m garage openings and sloped connections to the road; cars do not enter/exit them in the current route demo.

Ground-floor walls are actual aperture shells around a recessed core. Storefront windows contain shallow geometry for shelves, goods or reception counters. Door leaves, frames, handles, transoms, sign fascia, canopy brackets and paving are independent details baked into each building's static mesh. Cafe furniture stays in the frontage strip and lateral pedestrian variation is restricted to keep that strip clear. Runtime input/motion ownership is unchanged; rendering retains shared GPU buffers and cached draws. The exported district is approximately 11.42 MB / 108k triangles. Facade atlas storage is approximately 4 MiB including mip levels, versus 8 MiB previously (other textures and render targets excluded).

City Block now joins the northern apartment/office frontage at a party wall and keeps 1.0m/1.4m walkable service alleys. Four additional GPU-driven pedestrians use the alleys; geometry audits check the complete routes, building footprints and outward door/garage corridors. The original generated asset is currently about 12 MB.

The viewer renders autonomous district animation at up to 60Hz while retaining display cadence for input/drag/auto-rotation. Time still advances on every display callback; this reduces redundant frames rather than slowing actors. Opaque GPU bundles also omit repeated pipeline/material/camera bindings. Native CPU/Metal measurements, scope and replay commands are in [the layout/performance report](../../docs/design/city-block-tight-layout-performance-20261021.md). `scripts/summarize-metal-trace.py` filters one PID and merges overlapping GPU intervals; its activity percentage is not whole-device utilization and Metal memory overlaps process unified memory.


City Block 支持 `Lighting → Day / Night` 或 **N** 切换昼夜，命令行 `--night` 可用于夜景快照。四栋楼分别使用确定性随机的窗景：部分窗内有站立、坐着或看书的室内人物剪影，百叶窗与家具遮挡保留；夜晚部分窗户、招牌和路灯自发光。人物是 GPU 计算的原创窗内剪影，可走动、坐下、阅读；不是可进入房间中的活动 NPC。切换只更新通用 GPU 光照参数，不重建资产；自主动画默认上限 60 FPS，交互由原生显示同步回调驱动；当前 macOS host 最高请求 120Hz。实际帧率受屏幕、系统调度与 GPU 负载影响，移动到不同刷新率屏幕时无需手动切换。`Animation → Limit to 60 FPS (H)` 可限制街区自主动画与自动旋转到 60 FPS，再按 H 恢复自动；`--benchmark-120hz` 仍保留用于指定 120Hz 的测试上限。右上角 FPS 为已取得 drawable 并提交的帧率，不是屏幕标称刷新率。昼夜与系统 UI 主题独立，夜间 HUD 保持可读。可在暂停时切换，恢复播放后沿原时间继续。


夜间生活：四栋楼的入住率在 `assets/generate_city_block.py::BUILDING_OCCUPANCY` 分别配置，当前均以 50% 演示。已入住/空置分配固定，昼夜共用；空置窗户保持暗淡、没有人物且永不亮灯。已入住房间再按独立节奏和作息决定开灯，切入夜晚从 20:00 开始，60 秒相当于 1 小时，午夜后多数熄灯，保留少量夜猫子。暂停冻结作息与动作，R 复位，重新进入夜晚重启夜间作息。

施工楼使用单独的无住户、无窗户自发光材质；两个工地聚光灯照明施工区，并随围挡撤场淡出/拆除。六盏路灯照亮路面；所有轿车、警车、救护车、消防车、垃圾车都有夜间前灯与红色尾灯，前灯光锥跟随 GPU 路线转向。树、行人、车辆和楼体使用粗粒度椭球投影代理形成地面遮挡。通用局部灯光、路线运动、窗景和城市配方保持分离，动画不逐帧上传顶点、实例或灯位。


实例随时间改变状态：施工/撤场使用独立时钟，入夜冻结进度，回到白天从原进度继续；人物、车船、树木、水面继续。32 位行人分散在入夜约 25–245 秒离场，保留 5 位夜间行人；这是可见性时间线，尚未加入“找家门并走进去”的行为规划。午夜后雨棚灯按打烊曲线熄灭，门口双墙灯降至 35%，主路灯降至 55%，两盏次要滨水路灯关闭。灯具自发光和投向表面的局部光同时变化。每栋楼的墙灯与下照灯沿自身入口法线放置，灯光配置与模型从同一份生成数据导出。

时间机制分为：城市 JSON/作息和施工暂停累计（数据与宿主层）→通用时钟/可见性/调光参数（Scene3D API）→实例变换、人物淡出、光照和窗景（wgpu）。没有逐帧 CPU 人物/车辆/灯位计算，也没有像素往返。暂停、恢复、R 复位与昼夜切换沿用同一时钟生命周期。


窗帘同样是窗格实例的 GPU 时间状态：部分已入住房间分批缓慢拉合双侧布帘；帘布有柔和褶皱，遮住人物与家具轮廓，同时透出低强度室内光。空置房间不会因此发光；远处降低褶皱对比，避免细条纹闪烁。不新增窗帘动画帧贴图或逐帧 CPU 几何。


施工外壳增加独立 `construction-dust-screen` 节点：四面防尘织网和原创虚构 TEKES BUILD / BUILDING TOMORROW 品牌横幅。屏障沿用通用 GPU 分层搭高、完工后逐层拆除，与脚手架共享施工时钟；夜间停工一并冻结。使用单张 128×128 MASK 网布与一张 384×64 横幅，新增112 个索引顶点 / 56 个三角形，避免透明实例排序；不逐帧上传网布或横幅几何。
## Background performance measurements

The standalone viewer's Animation menu includes **Background benchmark (B)**;
the CLI equivalent is `--benchmark-background`. This explicit test mode keeps
animation and rotation running after keyboard focus moves to another app.
It defaults off, preserving normal focus-based pausing and reduced-motion rules.
Keep the 3D window visible at a fixed size, for example on a second display.
Minimizing, fully covering it, or switching away from its desktop can stop native
GPU presentation and invalidate a sustained rendering measurement.

## Frame rate and display refresh

The viewer defaults to a 60 FPS ceiling for autonomous animation and automatic rotation. Display changes still adapt automatically up to that ceiling. **Animation → Limit to 60 FPS (H)** is checked by default; press H to allow native display cadence (the macOS host requests up to 120 Hz). Actual FPS depends on display scheduling and GPU load. `--benchmark-120hz` sets a fixed 120 Hz test ceiling.

The upper-right HUD updates twice per second and counts acquired, submitted drawable frames rather than animation callbacks. Its milliseconds show average frame interval, not GPU execution time or display scanout timing. Idle resets the sample without running a separate HUD timer. Numeric labels do not accumulate in the text cache.

Native 60-second near/far 120 Hz measurements are in [the report](../../docs/design/city-block-120hz-20261021.md); screen switching checks are in [the automatic refresh report](../../docs/design/city-block-auto-refresh-20261021.md). Those native results use the full working demo assets and city/night WIP, not only this scoped commit.

## Ground railway and station

City Block adds an original E235-inspired three-coach demonstration on two opposing ground tracks, with rails, catenary and an island platform. `assets/city_transit.py` authors the infrastructure, `assets/city_train.py` authors coach/cab/door/interior geometry, and `assets/city_train_liveries.py` keeps livery colors separate for later variants. No third-party train models, logos or photographed textures are bundled. This is a three-coach visual demo, not the real eleven-car formation.

`src/transit.rs` supplies immutable instances; generic Scene3D GPU schedules perform dwell, acceleration, braking and door motion. `src/station.rs` derives a bounded crowd from occupied residents/jobs and supplies boarding/alighting routes. Passenger body/limbs share GPU route timing. Platform routes remain level until the exit stairs, and coach gangways use opaque bellows and connecting floors. Whole coaches fade at the route window boundary rather than exposing a clipped cross-section. Only glass uses alpha blending; windshields use a separate darker material.

Transparent instance order is resolved on GPU within each primitive (up to 64 instances); primitive groups retain the renderer's existing camera ordering. This is not global order-independent transparency for intersecting glass meshes. The shared interior geometry includes longitudinal seating, seated and standing figures, and night-emissive LED diffusers and surfaces.

## Scene panning

**Shift + left drag** or **middle drag** moves the entire displayed scene; **Shift + arrow keys** moves it in small steps. Option + left drag / arrow keys are aliases. Plain left drag orbits the camera, right drag rotates the model, scroll / + / − zoom, and R resets the view including pan. The View menu exposes the four pan directions. Pan distances are normalized by the 3D viewport, so the scene follows the pointer consistently across window sizes and display scales. `Camera3D::pan_by` is a reusable, platform-independent screen-space camera offset; only camera uniforms change, with no geometry or instance uploads.

The minimum camera zoom multiplier is 0.04 (previously 0.2), allowing five times more magnification at the close limit. Scroll or + continues zooming in; Shift drag can bring off-center details into view, and R restores the fitted home view.

Train wheels are separate shared wheelset meshes (`transit-wheel-0..3`), including axle, circular tires, brake discs and visible hub bolts/inspection marks. Bogie frames and axle boxes stay fixed. Generic `Rolling3D` composes with route/Transit motion: local X rotation uses travelled distance / scaled radius, rotating both vertices and normals on GPU. Stops, acceleration and braking use the same integrated distance as the train. The four wheelset batches add 24 instances for six coaches and reuse spare GPU instance channels, keeping the 192-byte record unchanged and adding no per-frame CPU transforms or uploads. Road wheels reuse this same rolling API; see below.

The ground railway and island station now occupy the strip between the eastern road and the river. Water, the quay and boat navigation route move together to preserve clearance. The independent `city_clouds.py` / `clouds.rs` module adds five shared cloud instances drifting on a GPU route. A descending projected-size fade (`ScreenDetail3D.fade_pixels`) keeps clouds hidden up close and smoothly introduces them when zooming out; no per-frame CPU cloud movement or geometry uploads are added.

Road wheels (`city_road_wheels.py`, `road_wheels.rs`) now use the same GPU rolling path as trains. Sedans/police cars and ambulances have two axles; fire and garbage trucks have three. Each wheelset clones its body instance route, phase, speed variation, scale and optional route schedule; only the wheel radius/axle and neutral tint differ. Turning speed therefore follows the exact integrated body travel, including speed changes, zero-speed parking and paused scene clocks. Vertices and normals rotate together on GPU. Six shared wheelset meshes replace static tires inside vehicle bodies; road wheels use the same GPU LOD thresholds as the bodies, and do not add lights or shadow proxies. No per-frame CPU wheel simulation or instance/vertex uploads are added.

Passenger side glazing now has a 0.24-unit (0.96 m at authoring scale) height, wider solid piers and a taller sill/header; glass area is approximately 30% lower than the initial lit-window model. Door panes shrink about 25%. These are visual model proportions guided by JR East exterior photos, not asserted manufacturer dimensions. Night passenger-window emission remains enabled.


### 街区视角 tabs

街区顶部提供总览、行人、汽车驾驶、电车驾驶、电车乘客五个视角；也可按 `1`–`5` 或使用 City viewpoint 菜单。各 tab 共用同一城市时钟，切换不重置施工、车辆和灯光进度。跟随模式拖动/方向键转头，滚轮调整视野；空格暂停城市和镜头，`R` 返回总览并复位。

例如：`cargo run -p tekes-ui-3d -- --demo city-block --view passenger --night`。`--view` 可选 `overview`、`pedestrian`、`driver`、`train-cab`、`passenger`，可与 `--time`、`--snapshot` 组合验收。

相机由 Scene3D 的 `Camera3D::look_at` 和预编译 `RouteSampler3D` 驱动。CPU 只采样当前视角的一到两条路线，模型、车轮、乘客与城市动画继续在 GPU 上运行。进入车内时通过缓存的透明材质发光倍率关闭外观窗玻璃光晕，保留不透明车厢灯具及城市照明；切回总览恢复外观发光。

当前是有限街区演示：电车驶出已建模区间时优先跟随另一方向仍在区间内的列车；两列均离开后转到站台候车，重新驶入时自动跟随。驾驶模式为自动沿原车辆路线行驶，未实现键盘驾驶或碰撞导航。
