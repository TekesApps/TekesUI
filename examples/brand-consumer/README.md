# Consumer-owned BrandConfig

当前交付方式：同一精确 TekesUI Git revision + 消费方持有的离线 design artifact。开发时可显式 path 指向完整 WIP；没有发布 crate，也不把未提交 WIP 写成已发布 revision。运行时仍是纯 Rust，无 JSON/Python 编译器依赖。

消费项目保留自己的 `BrandConfig.json`。先在依赖 checkout 执行显式离线编译（Python 使用提案 requirements），输出到消费方新目录：

```sh
python3 /absolute/TekesUI/scripts/compile-consumer-brand.py \
  --config /absolute/MyApp/BrandConfig.json \
  --output /absolute/MyApp/offline-design
TEKESUI_DESIGN_ARTIFACT=/absolute/MyApp/offline-design \
  cargo run --locked --manifest-path /absolute/MyApp/Cargo.toml -- /absolute/MyApp/preview
```

本目录本身是独立 Cargo workspace 模板。复制到仓库外，将 Cargo.toml 中依赖改为相同的 `git` + 精确 `rev`（所有 TekesUI crate 必须相同），或明确的开发 path。模板包括锁文件，最终项目需在依赖方式改变后更新自己的锁文件。`--check OUTPUT` 只验证公共 API 与真实 Button 场景；默认输出还要求真实 GPU，生成四主题、@1x/@2x 场景 PNG。没有打开新原生窗口；不等价于原生交互或 VoiceOver 验收。

`Palette::from(theme)` 使用消费方编译品牌。Gallery 的显式索引用于预览；`Palette::new(theme, index)` 对未知品牌返回 None，未知 role 同样返回 None。传入已有结构字面量的无效索引也不会静默回到默认品牌。控制绘制对缺失语义明确失败，消费方应在配置/选择边界检查结果。

artifact 是现有完整 token 投影和其作者输入的离线副本，只有副本中的 `design/brand/config.json` 被替换；TekesUI checkout 不被修改。编译使用原品牌算法、policy、角色图及状态/对比检查；不另写色阶。Rust 构建使用既有校验器验证 artifact 自洽、精确输入哈希、API 格式与相同依赖的设计版本。未知配置键/角色、错误主题、未知 schema、旧/不匹配输入、缺失文件、相对 artifact 路径均报错，不降级到默认品牌。未设置变量时维持原默认。变量改变与 artifact/依赖输入改变都触发 Cargo 重建；多个消费项目的品牌应分别构建，当前不提供同进程动态配置加载。

升级 TekesUI 后必须从新的精确 revision 重新生成 artifact；它不是跨版本通用协议。保留旧 artifact 供旧依赖回滚，不修改原始导入包。交付物不包含 Python 运行时，离线生成环境单独安装固定 requirements；库构建不启动 Python。

`python scripts/test-consumer-brand.py` 在仓库外创建两个独立工作区，持有不同 BrandConfig，依赖同一当前代码；分别构建真实控件场景，验证四主题状态与中性/业务角色隔离。CI 已有 scripts/test-*.py 入口会持续执行。设置 `TEKESUI_BRAND_GPU_EVIDENCE` 为不存在的新目录可输出同样场景的实际 GPU 证据。该验证不测试性能。
