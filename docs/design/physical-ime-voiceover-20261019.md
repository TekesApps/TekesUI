# #17/#20 物理拼音与 VoiceOver 现场复验

使用全部最新WIP对应的公共候选4a02d30bd65c1117f433a874a7c421492bf76ad101e92f7260ecd470323554a1，188个公共文件和AgentKit原有冻结均逐文件保护。316文件消费者源码复制后仅增加上一轮相同的两个只读审计观察：demo native事件状态、platform selftest输入上下文；audit-only.patch与audit-inputs.json完整保留。没有改变生产分发／文本状态机／AX实现，没有新增共享抽象、token或组件迁移。实际测试二进制SHA6626a178816bf423844eddcd8920684ee1c95c25e863369abf0375ca64f98008。

外接屏U2790B、2×、当前Light；物理系统SCIM.ITABC拼音和实际Metal消费者分开记录。按键日志验证：

- n/i产生Preedit；Enter在组字开始状态下仅一次Commit("ni")，无命令回调、无换行／业务发送。
- 空格一次Commit("你")；用户确认候选出现并选字成功。
- 组字中Tab产生Preedit("nī")，marked 0..2保持，没有insertTab命令；用户确认候选切换。
- Escape产生空Preedit并清除marked；用户确认候选消失、窗口保留。

四条路径均保持Composer焦点4、Transcript条目20不变。Tab在没有组字时插入制表符是既有显式Insert配置；不能把Tab当作系统切换输入源的默认键。先前“Enter应选出你”的指导不准确，已向用户纠正。[Apple候选选字说明](https://support.apple.com/en-ke/guide/chinese-input-method/cim12992/mac)、[系统切换输入源说明](https://support.apple.com/en-gb/guide/chinese-input-method/cim119a8d473/104/mac/26)。本轮未更改系统设置。

VoiceOver真实进程启动后停止桌面自动操作，要求用户仅点测试窗口标题栏，不点正文，用VO左右到“会话记录”，依次VO Shift Down、VO L、VO Down。用户完整答复“voiceover works as your expecting”，据此将独立进入／当前行／下一行三项记为人类现场确认通过。原生AX另证最终focused owner为Transcript3，AX查询不替代听读反馈；未录制音频，不声称自动听取。此前失败结果保留为历史，当前4a02候选的此轮实际消费者复验通过；不能把旧版本失败原因倒推为已证实的特定根因。

公共API沿用同188文件候选已封存的43 platform／130 independent／27 UI／76 headless、Clippy及GPU证据，本轮没有生产源码改动，不重复整套自动测试。真实接入是实际AgentKit demo；Gold7 G5编辑阅读、G6组字消费及无障碍操作、G7可复现输入和消费者证据得到上述范围现场补证，未宣布完整Gold7矩阵通过。适用于当前共享TextSurface和现代AppKit文本桥接；验证样例为真实Composer与readonly Transcript，其他Gallery publisher未被本轮扩展验收。

仍未覆盖：#18物理组字时外部同步冲突、#20物理候选窗在resize/clip/backing-scale中的几何及完整主题矩阵；公开不可变pin与AgentKit正式候选提升单列交付阻塞。性能按用户要求不测。未提交、推送或关闭Issue。

结束时VoiceOver通过活动监视器正常Quit，ps确认不存在；测试窗口关闭，原wake与clone均Closed，运行时与7项精确归属构建产物经fresh lsof核验后清理；共享依赖缓存、源码、脚本、日志、截图与反馈保留。证据位于design/reviews/physical-ime-voiceover-20261019，原生轮次design/reviews/component-runs/agentkit-physical-text-20261019-132138。
