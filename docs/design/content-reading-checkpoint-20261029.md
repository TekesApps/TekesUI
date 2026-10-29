# 跨新 ContentHost 的纯值阅读检查点（#42）

本增量沿用 `content-reading-anchor-20261020.md` 的布局引擎，不引入业务 IO、Agent 类型或第二套 offset 补偿。`ContentReadingCheckpoint` 的 format 1 只有文档 key、revision、following 或稳定业务 item key 的 item-local 逻辑点、viewport 内 y 和旧已加载邻居优先序。没有 ComponentId、地址或 attachment；宿主可自行编码这些公开纯值。未知 format、空 key、重复邻居、非有限／负 within 均拒绝。SDK 接收反序列化后的值时统一验证，宿主编码器仍须报告语法／存储失败；absent 与 unavailable 不能混成默认 bottom。

## 接入与修订策略

- 成功布局后，`capture_scroll_reading_checkpoint(owner, &current, &keys)` 导出。`keys` 是业务 String→本树 ComponentId 的一一映射，必须覆盖所有声明项，包括尚未加载项。未有效布局、旧版本、错误文档分别拒绝；后续布局失败后，旧帧不能用于导出或恢复，须先完成正常布局。
- 构造全新树并附加文档后，在异步存储／加载工作开始之前调用 `prepare_scroll_reading_restore(owner, &current)`。它持有当前运行期 token 与输入意图，不能持久化。先完成目的树的正常布局，再调用 `restore_scroll_reading_checkpoint(&request, &checkpoint, &new_keys)`。旧值不授予新树操作权限；不可把迟到请求的旧 token 或 request 换成刚读出的当前值。
- 文档 key 必须相同；检查点 revision 可等于或早于目的文档，未来版本拒绝。当前 request 必须精确匹配目的版本；文档更新、A→B→A、同 key 重新 attachment 都拒绝旧 request。
- 映射中不存在的 key 表示业务已删除：按检查点保存的已布局后继、前驱，再当前第一个已布局项回退（跳过尚未加载的邻居），结果标记 Fallback。映射中仍存在但未有效布局的候选表示尚未分页加载：返回 ItemUnavailable(key)，不安装任何状态、不声称可见。宿主保留原纯值并加载历史。若加载推进 revision，旧 request 失效；宿主必须按自己的业务操作重新授权恢复，不能让迟到后台任务刷新身份后抢过新的用户输入。
- SDK 下一次正常 render 在绘制前解析几何、恢复与钳制。固定宽度保持相同 item-local 点；宽度／字号变化保持逻辑点并在 item 缩短时钳制，不声称保持同一个字。following 显式恢复至最新；history 后追加仍不跟随。

## 输入与原子边界

request 持有不透明输入身份。原路由的滚动、只读后代手动历史意图、显式 follow/stop、另一次 restore 都换掉身份；已有显式 reveal 身份同时验证。即使输入后已经 render，迟到 restore 也拒绝。成功安装后、render 之前的新滚动清掉 pending，原输入优先。旧动量 fence 沿用现有逻辑。

所有可失败的纯值、映射、token、布局和可用性检查先完成；安装阶段没有 callback 或 IO，只改变原 reading/scroll 状态，不改文本、焦点、选区、preedit 或 undo。失败保留原 token 与全部状态。成功会消耗该 request 的恢复权，重复调用拒绝。

## 验证边界

独立 `examples/composition-consumer/tests/reading_checkpoint.rs` 通过公开接口和自有业务映射测试：混合 Content 中段捕获、JSON 存储纯值、销毁旧树、新 ID 树恢复；following/history、前插追加、宽度／字号 reflow、删除／暂未加载、坏值、错文档／旧 attachment／新输入以及 editor 连续性。普通 renderer 与 CoreText 的 API 检查分开记录；原生窗口、真实输入和像素结果不能由纯值或程序化事件测试替代。原始命令、source identity、结果和未验项由本批收据记录。
