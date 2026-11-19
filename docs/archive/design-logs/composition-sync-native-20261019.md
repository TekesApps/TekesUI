# #18 物理组字期间外部同步

最新全部WIP与公共4a02d30候选188文件一致；AgentKit主checkout128文件及原冻结保持。根因分类为公共文本编辑事务与消费侧回填契约；现有reject契约足够表达，本轮没有生产缺口或新增抽象。

测试专用小宿主在真实物理ni Preedit后一次性调用现有公共apply_external_value及真实RunSurface.sync／RunController.edit：重复确认Unchanged且baseline保持；新外部值Conflict(Composition)，真实消费侧回填明确Err，不隐式commit/cancel/preedit。物理空格确认“你”后，公共旧请求Conflict(Revision)，旧消费快照Err(stale or unversioned)，当前权威版本由4推进至6且payload你，选区1..1／焦点4／条目20保持。没有内部队列或静默覆盖。触发器不伪造Preedit或Commit，正文和候选来自用户物理输入。用户确认候选未被打断、正常得到“你”、没有REMOTE替换。

公共API独立controlled_text13项、真实AgentKit controlled_editor5项、审计宿主strict Clippy通过；原有完整候选自动／GPU证据沿用，不把本轮机器assert当人类体验。Gold7 G5编辑数据保持、G6同步冲突与G7公共API／真实消费者分层补证；实际外接屏U2790B／2×／Light。合理例外：审计专用一次性驱动只在隔离消费者复制中，三文件完整audit-only.patch及317文件输入清单保留；实际AgentKit原目录和生产TekesUI源码不变。未扩大Gallery、物理会话A/B矩阵或性能范围。

窗口关闭时原wake和clone均Closed；11项精确归属产物经fresh lsof后清理，共享依赖缓存保留。VoiceOver未开启；不提交、推送或关闭Issue。#18本轮物理组字冲突及旧请求失效补证通过，公开pin与AgentKit正式提升仍单列交付阻塞。

证据：design/reviews/composition-sync-native-20261019/native-result.json、checks.json及component-runs/agentkit-composition-sync-20261019-133811原生日志、截图。

输入源边界：物理会话开始记录为SCIM.ITABC，结束记录为ABC；本轮没有程序化切换输入源或写入系统设置，保留用户当前输入状态，差异单独记录于input-source-boundary.json。
