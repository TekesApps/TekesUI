# #23 公开 CI 修复候选：独立消费者复验

用户已明确授权本次8文件提交、推送和交接；本次不合入main、不打tag、不关闭Issue。当前Gallery全部WIP、此前正式431冻结及037/122历史原生证据保持各自身份。

- Repository: https://github.com/TekesApps/TekesUI
- Exact public commit: `8cf0840790334ba224afc0595897c74231c712e6`
- Branch: `codex/agentkit-public-pin-issue23-20261020`
- Parent: `61ddf7c68fccd819e20a01ba3104a8a0f8d7e64f`
- Reviewed source identity: `53228e613cbd4c5da0d398a45c512c10fdab93a81419497aa2515eef11d6a53a`
- Complete1825-file source manifest: ../issue23-ci-followup-20261020/manifest.json
- Portable reviewed archive SHA256: `2519ba6c697f0a9f7f6600821faf01f451feb98c5a9a99f37d4a80d392d9ed15`
- Public input manifest: `docs/agentkit/public-source-issue23.json`,201 declared inputs, identity `ae0b1055118d50d4d6e28a3d47e5e40bab254c7c6721d0c483e2a49f78114a4e`
- Remote CI: https://github.com/TekesApps/TekesUI/actions/runs/39365340268 (**completed/success**; all steps passed, final receipt in CI.json)

精确改动：相对61dd仅8文件，原始431基线累计20文件。补齐既有TextSurfacePresentation生成绑定；复用已公开PR24的隔离语言参考测试、CoreText shape测试及必要测试依赖；CI Clippy使用--locked。公共生产库与native consumer字节不变，未夹带后来Link、ButtonContent居中、required等当前Gallery WIP。所有公开1825源码文件与已验证532包逐一匹配；公开checkout干净，远端分支HEAD匹配。

## 独立复现

在自己的隔离git checkout解析精确git+rev，而非相邻WIP路径、移动分支或仅同名版本。记录git rev-parse HEAD、状态、cargo metadata/lock实际解析和目标crate源码根。先运行：

```sh
python3 scripts/verify-agentkit-public-pin.py docs/agentkit/public-source-issue23.json
```

应确认201项输入及上面的ae0b输入身份。源码包准备阶段已通过923项workspace、134项consumer及两处严格all-target Clippy；原1项ignored和3项用户延期性能测试不记通过。具体可复现命令见 ../issue23-ci-followup-20261020/checks-final.json。CI遵循相同延期边界；不测试性能、不修改系统设置或golden图片。

请运行AgentKit会话现有真实消费者构建/验证流程，并针对#23确认公共无表单装饰文本的实际接入：选择/复制、滚动/阅读位置、焦点/原生语义及原Issue其余适用条件。旧原生/听读证据保留旧版本身份；如复用，说明生产输入字节等价边界，不声称在新CI提交上重新做过物理IME/VoiceOver。不要覆盖既有正式冻结包或发布pin。

输出独立结论：精确commit和实际解析输入、构建与消费者结果、#23逐条件满足/未满足、是否技术接收及是否建议提升开发冻结候选。任何新失败明确保留；当前发送方不会合入main、打tag或关闭Issue。请在本会话记录结论，发送方会读取。

## 运行时闭包口径补充

完整公开输入清单201项（ae0b1055）还包含Gallery的Cargo.toml与snapshot.rs测试来源；它们不属于AgentKit实际八包运行时闭包。消费方已另存199项运行时清单（f0af230a），其余每项哈希与完整201项相同，同时保留完整201项独立校验。不得仅从完整清单中所有Cargo.toml推导运行时包数，也不得通过跳过哈希或额外源码检查消除不一致。双方各自的清单身份及精确差集见consumer-manifest-scope.json；正式431工作区未变更，最终消费者结论仍待返回。
