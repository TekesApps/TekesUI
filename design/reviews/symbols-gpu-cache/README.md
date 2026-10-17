# 图标 GPU 缓存与匀速旋转实测

**CPU 与构建成本下降，稳定 120Hz 零掉帧仍未通过。** 本轮在共享桌面顺序测量，观察到其他项目 rustc 编译及 du 磁盘扫描；结果不构成安静环境下的随机化 A/B。原始失败数据全部保留，不以 p99 或平均耗时替代实际呈现验收。

## 实现与视觉验证

- SVG 解析、分层、原图和高分辨率图集缓存。几何与透明度在 GPU 做缩放、旋转、位移、混合，复用纹理；路径描绘与有共享轮廓的 Magic Replace 继续动态光栅化/原位上传。
- 新增外部时钟驱动的 `LinearRotation`：恒定角速度，无缓动；调速、反向保持角度，暂停不积攒时间。Symbols 第15项，默认2秒一圈；不是 Apple 内置效果的兼容声明。
- 64个图标/主题/倍率静态完成态与原 SVG 严格相等；120个运动组合覆盖面积差<3.5%、重心偏差<0.6设备像素。运动中图集插值不声明矢量逐像素等价，图集增加显存占用。
- Motion 内核57项集成+1doctest，Gallery Motion31项及perf8项通过；GPU glyph/image18项、offscreen23项、严格Clippy通过。最终纯Rust guard563文件通过。

## 环境与口径

Apple M4，内建 Retina 120Hz、2×，FIFO、3 drawables。正式组GPU时间戳关闭，独立诊断开启。每组预热1.5秒，结束暂停全部模型并等待5秒检查idle。CPU100%=一个逻辑核心的user+system。实际呈现长间隔>1.5刷新，即12.5ms。全组模型错误0、结束暂停成功、idle额外动画回调0；idle CPU0.025%～0.94%（最高为第一组）。

冻结签名二进制SHA256：`75114371cb3d7e84b5725c0d0d69786cbf0366e60fb0261f1d19feafd09ef3c9`。本轮旧版二进制/报告未覆盖。源码与签名见build-measured.json，后续主控displaylink/收据修正等源码变化见source-drift-after.json；这些变化不自动继承本轮结果。

## 结果

| 场景 | 秒 | CPU均值 | 建场景均值 / p99 ms | 渲染 / 回调 | 实际长间隔 | 最长 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 3实例 / 呼吸 | 30 | 10.62% | 0.024 / 0.053 | 3516 / 3516 | 78 | 20.833 |
| 24实例 / 混合持续 | 60 | 19.32% | 0.847 / 1.822 | 7069 / 7069 | 99 | 29.166 |
| 24实例 / 连续调参 | 60 | 16.41% | 0.977 / 2.290 | 4521 / 7135 | 25 | 25.000 |
| 24实例 / 中途切目标 | 60 | 11.35% | 0.823 / 1.881 | 1540 / 6882 | 66 | 25.000 |
| 72实例 / 调参及切目标 | 30 | 36.64% | 2.359 / 5.432 | 3252 / 3252 | 298 | 25.000 |
| 42实例 / 全14效果 | 20 | 26.16% | 1.427 / 2.739 | 2400 / 2400 | 1 | 16.667 |
| 24实例 / 固定纹理诊断 | 30 | 7.89% | 0.027 / 0.058 | 3598 / 3598 | 3 | 16.667 |
| 24实例 / GPU计时诊断 | 30 | 22.44% | 0.925 / 1.990 | 3506 / 3506 | 70 | 20.833 |
| 24实例 / 匀速旋转 | 60 | 10.09% | 0.030 / 0.066 | 7138 / 7138 | 64 | 20.833 |

**收据限制：冻结版 harness 在跳过渲染时复用旧 drawable serial。** 调参组2614次、中途切目标组5342次回调未产生新渲染；报告原始 `not_presented` 2655/5385可能重复旧收据，不等于这么多个独立drawable。渲染标记原文保留，summary额外列rendered_callbacks/total_callbacks/skipped_render_callbacks；不改写旧原始JSON。这两组CPU不用于宣称同等已呈现负载下的提升。其余各组rendered=callbacks，但未呈现回执和长间隔仍须结合原文看。主控正在修正收据归属和记录窗口可见性，之后独立测试显示队列/驱动。

## 与旧实现的有界比较

旧结果在 `../symbols-performance/`。24实例混合持续：CPU46.24%→19.32%，建场景均值3.208→0.847ms、p993.624→1.822ms。72实例组合压力：CPU98.90%→36.64%，建场景p9910.800→5.432ms。42实例全效果：CPU62.38%→26.16%，建场景p996.107→2.739ms。两轮不同时间、后台负载不同，不把这些比例当成受控因果结论；阶段计时与上传计数共同支持重绘成本已下降。

3实例呼吸及24实例匀速旋转在测量期均为0次光栅化、0次纹理创建、0次上传。24实例混合每回调6次动态重绘/上传（两类复杂效果各三scope），72实例为18次；其余效果复用图集。匀速旋转60秒CPU10.09%、建场景均0.0296/p990.0663ms、暂停idle0.029%且callback0，仍有64次实际长间隔、最长20.833ms。固定纹理组仍有3次长间隔。以上不能宣布显示节拍问题已解决。

GPU时间戳诊断24实例：均0.722/p993.520/max7.550ms，仅本进程render pass，不含系统合成器或drawable等待，正式组未测GPU耗时。后续优先定位队列等待、显示调度及窗口可见性；路径描绘/变形仍有动态上传成本。

## 复现与原始证据

`raw/*.json.gz` 无损保留9组完整逐帧JSON，`raw/manifest.json` 提供原文/压缩SHA256。完整日志及启动时进程清单在本地 `target/symbol-perf-cache/`（进程清单不入归档）。`summary.json` 保留所有失败，`strict_zero_drop_pass` 全部为false。脚本退出0仅表示完整运行、没有模型错误，不表示性能通过。

```sh
python3 scripts/measure-symbols-matrix.py --binary 'target/TekesUI Symbols GPU Benchmark.app/Contents/MacOS/TekesUI-Symbols-GPU-Benchmark' --prefix cache- --out target/symbol-perf-cache
python3 scripts/measure-symbols.py --binary 'target/TekesUI Symbols GPU Benchmark.app/Contents/MacOS/TekesUI-Symbols-GPU-Benchmark' --sources 8 --mode steady --seconds 60 --label cache-linear-24 --linear --gpu-timing 0 --out target/symbol-perf-cache
```

调参/切目标调用真实控制器接口，但不是物理鼠标输入延迟验收。单次有限时长测量不能代表所有设备、主题、倍率或任意数量图标。旧版测量、用户常规预览、其他工作区修改均保留。
