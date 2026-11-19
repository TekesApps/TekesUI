# #20 活动组字在重排和裁剪中的可见几何

基于全部最新WIP，先完成#18物理同步验收。随后#20第三轮物理组字保持至4/4且人类未见明显候选错位，但原生日志证明200%+祖先裁剪+720×600时caret不在有效viewport内；native查询只是正确转换了离屏caret，并不能证明viewport一致。该轮保留为部分证据，已关闭清理；前两轮准备条件和提前确认的限制亦完整保留，不覆盖历史。复用已有能力整改，未另建Issue。

根因：公共TextSurface.layout_with按未裁剪的ContainerLayout.content计算排版/滚动，重排的阅读锚点可能留下离屏组字；真实AgentKit AX适配忽略原语义投影已有visible字段，使用了完整frame。没有缺少新token、状态机或框架。

共享修复在crates/tekes-ui-controls/src/text_surface.rs：复用ContainerLayout.visible_clip作为实际编辑/滚动内容视口，使排版、caret、命中和滚动使用同一可见区域；活动marked文本遇到字体或几何变化后，复用原scroll.reveal保住caret。外部field边框slot仍由既有容器提供，不加页面局部偏移。适用于现有TextSurface支持的TextField/TextArea/SearchField、只读阅读和公共组合消费者；未扩大组件迁移。

合理例外：完整遮挡时仍按既有不可见交互规则处理，不假造可见caret；没有marked文本时保留原阅读锚点；同布局下显式用户滚动不强制跳回caret。回归分别验证这些边界及原默认表单、阅读/滚动矩阵。

真实消费接入：新的独立可构建delivery-consumer-source沿用原316文件消费者，仅更新公共快照、AX frame使用n.visible、隐藏节点不发布，以及实际组合回归。AgentKit主checkout128文件和原冻结不变，consumer-adaptation.patch可审查；这不是正式消费方提升或公开pin。公共新候选03721866a362b1d31261ebbbc86140eb07e9e23320871c325234a5212591213d（188文件），相对旧4a02仅当前公共共享修复。

验证：新增独立公开API回归先红（150%caret落在viewport下方），修复后1×/2×、150%/200%、祖先clip、缩窗/恢复均绿，编辑器全量Debug状态不变；同布局用户滚动保持。公共控件233项、独立消费者131项、真实UI28项、headless76项，以及相关Clippy通过。新增实际RunSurface使用真实CoreText/public Field/Stack回归，不仅验证孤立surface。

原生自动脚本使用同一最终审计宿主，在实际U2790B、2×、非零窗口原点，以合成preedit覆盖全部五阶段；caret均在有效viewport，native firstRect与caret误差<0.02pt，实际AppKit AX frame等于可见viewport，marked/selection保持，最终确认和旧外部同步失效也通过。原生脚本属于合成组字，不替代物理候选窗听看；新候选最终物理复验已通过，详见下文。审计夹具改为记录layout实际应用phase，避免原先Atomic phase在绘制中变化导致标签错配。

Gold7：G5编辑/选区/历史与阅读位置边界，G6活动组字、裁剪/重排、原生几何与AX，G7公共API/真实消费者/复现源码分别留证。真实GPU与最终Clippy/边界检查见final-gates；完整主题/其他Gallery publisher与公开pin不自动算完成。性能不测，VoiceOver保持关闭，不提交、推送或关闭Issue。

证据：design/reviews/composing-viewport-fix-20261019；shared-fix.patch、consumer-adaptation.patch、红绿回归、checks.json、synthetic-native-result.json与逐文件输入清单。旧候选及审计源码不修改，新候选独立留存。最终物理验收后小宿主已关闭，135项精确归属构建产物经fresh lsof后清理，公共缓存保留。


## 最终物理复验与清理

最终公共候选0372186、审计二进制cc4caa558311ed15b2e1bcf15305d749462bc8911aa5ab1760e7cb60c713d004，与通过原生合成脚本的二进制完全相同。用户明确等待4/4后再空格，报告候选跟随正常、组字未打断、无REMOTE替换。原生日志五阶段均为活动key window、marked2205..2207／selection2207..2207／焦点4保持；150%、200%、clip、720×600缩窗及恢复时caret均在可见viewport，actual firstRect与caret误差<0.02pt，AppKit AX frame等于可见viewport。physical-native-result.json完整留证，不用截图替代人类候选窗观察。

同一物理ni组字触发最终候选#18真实同步复验：重复Unchanged，外部替换Conflict(Composition)，真实消费者Err且baseline不变；确认你后旧公共请求Conflict(Revision)、旧消费快照stale Err，当前本地文本不被覆盖。确认Space键链id3仅一次Commit你；4.6秒后的另一次普通Space为独立id4，仅一次普通空格，分别记录而非误判重复分发。

最终GPU SHA819c3c0d9415b7c17eacec9b47825dfd19c897e817c241e6b4a396d48ac05fc5与原基线一致，公共/交付/审计Clippy及Rust UI/headless边界通过。物理屏幕仅U2790B backing2；backing1是自动API证据，未冒称原生1×。VoiceOver本轮始终关闭；此前4a02听读结果保留为历史，不因本轮几何通过而重标为0372186听读验收。

所有验收窗口关闭，wake原件与clone Closed；135项构建产物按Cargo输出、哈希与lsof精准清理。保留源码、316文件可独立构建的delivery源码与tar归档、318文件审计源码、公开API回归和分层证据。当前#18/#20本轮范围已完成；公开pin/AgentKit正式提升、#20其他主题/文本复制及其他Gallery publisher仍按原台账单独列示，性能不测。不提交、推送、关闭Issue或发布。
