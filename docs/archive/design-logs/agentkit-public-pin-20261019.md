# AgentKit 正式 pin 交付与最终复验

本文保留当日过程记录；“运行中”“暂记待验收”及旧 #2 阻塞不代表当前状态。最新对账见文末。

本轮用户已授权独立提交、推送、正式 pin 消费切换，以及满足原条件后的 Issue 关闭。

公开提交：[431093786de47b48cd566d0d9c0eafa088b10d68](https://github.com/TekesApps/TekesUI/commit/431093786de47b48cd566d0d9c0eafa088b10d68)，分支 `codex/agentkit-public-pin-20261019`。交付基于 `80d6b05` 的独立 worktree；188 公共输入完全等于已验收的 037，相关 crate 无额外文件。182 个变更文件的来源、摘要和依赖范围在该提交 `docs/agentkit/delivery-changes.json`，未携带后续 Gallery 私有整改。

独立交付验证：workspace 923 通过、1 项原有忽略；第二公开消费者 131 通过；双严格 Clippy、设计及 Python 门禁、fmt、小宿主构建通过。首轮系统 Python 3.9 不满足要求的失败日志保留，使用 Python 3.13 和固定依赖后通过。这不是完整 Gallery WIP 1046 项测试的重标。

原生、物理 IME 和 VoiceOver 证据保留原输入身份，未声称重新进行实机验收；性能未测。

GitHub CI：[39217768350](https://github.com/TekesApps/TekesUI/actions/runs/39217768350)，当前运行中。AgentKit 会话已收到公开 SHA 和正式消费切换授权，188 文件比对已通过，正在保留旧 e4 并改为 git + rev，复跑三套测试、工具、两项故障回归及构建。七项暂记已实现待验收，待逐项最终结论后关闭；公开 pin 已不再缺失。

保护检查：交付前后原工作区 28034 个文件均未改变，原 HEAD 仍为 80d6b05；037 封存包和全部公共输入不变。随后只在原工作区更新本轮负责的账本状态和报告。独立交付分支已提交且干净。

验收后清理 163 个无占用构建文件，逻辑大小 402106485 字节，保留共享依赖、源码和证据；两套本轮 Python 临时环境已清理。本批未启动原生应用或 VoiceOver。完整本地收据在 `design/reviews/agentkit-public-pin-20261019/`。

公开分支尚未合并 main，没有打 tag 或发布注册表版本。

## 最终结果

AgentKit 已将实际 Cargo 消费切换为公开 git+rev 4310937，8 个包均解析到该 Git checkout；188 输入再次核对一致。保留旧 e4/.deps、manifest、lockfile及两处适配备份。UI30、独立公开consumer131、headless76、共享库287、工具16项、三套严格Clippy、demo构建、两项故障回归及真实Native SEP到GPU链路通过。正式报告副本在本批consumer-final/verification.{json,md}。

双方逐项比对GitHub最新62条原条件后，#15–#21均已评论留证并关闭为completed；时间和评论URL在closure-results.json。仅按这七项原范围关闭，不升级UI成熟度。条件跟进自动化已按既定规则暂停，消费会话已获通知可继续既定开发。

公开CI39217768350设计门禁、fmt、严格workspaceClippy通过；旧Gallery golden像素比较失败，后续CI步骤跳过。失败未当作通过，已在原#2补充[证据](https://github.com/TekesApps/TekesUI/issues/2#issuecomment-6262487097)，继续开放并核查；不改golden或扩大容差以消除失败。

等待消费复验期间另修复Gallery私有Menu/Popover失活残留呈现；只修改当前WIP，不混入公开4310937，详见gallery-transient-lifecycle-20261019批次。

## 2026-10-20 对账

实时核对确认 #15–#21 已关闭，正式 pin 仍为 4310937。原账本中的早期“尚无公共契约／缺公开 pin”等说明已归档到各项 verification_history；当前 verification 引用正式消费收据和原关闭评论，不扩大原 Issue 范围。

#2 后续通过已授权的 PR #24 修复并关闭：main `dbe5f72170c26bc1d4c227f8274bb922e3dbf1ca` 的 [CI 39228327983](https://github.com/TekesApps/TekesUI/actions/runs/39228327983) 已成功。此前 431 分支的旧 CI 失败仍保留，两个源码身份不混同，也不据 main 通过宣称全部当前 WIP 通过。

#22/#23 是独立新增条件。#23 的最终最小候选为 `122ae31df7e506ff85b492608550dc0bc0d9fa8cb803382b5d66e2e276511cd4`，已完成公共 API、消费者、GPU 和全新解压回放；原生激活后的复验仍待完成，未提交、推送或提升正式 pin。

实时快照及对账收据：`design/reviews/ledger-reconcile-20261020/`。当前根工作树 context 仍拒绝过期 token 观察并保持 stdout 为空，未将过期报告当作当前能力清单。
