# Typing：时间预算与柔和浮现

研究与实现日期：2026-10-16。

## 官方资料核对

用户提到的 Cloud Code 按 Claude Code 方向检索。[Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) 与 [terminal configuration](https://code.claude.com/docs/en/terminal-config) 能确认终端产品与流式输出，未公开本文效果对应的淡入CSS、字速或曲线。TypeScript本身不能证明采用CSS渲染：终端、Web、IDE面板的渲染路径不同。本轮未采用泄漏/反编译代码，也不将第三方猜测当作官方实现。

可确认的公开方案是 [Vercel AI SDK smoothStream](https://ai-sdk.dev/docs/reference/ai-sdk-core/smooth-stream)：将输入分块与输出节奏分离；官方文档给出默认10ms分块延迟，并推荐CJK采用Intl.Segmenter做语言相关切分。这是流处理参数，不是Claude的淡入规格，也不等同于单字视觉动画。

在Web里，视觉层可以用CSS opacity animation，例如从0到1；它仍然不能决定何时释放多少输入文本。TekesUI继续采用纯Rust时间模型与原有Scene字形绘制，不引入CSS/JS运行时。

## 两层职责

1. **TypingEngine**：接收已到达的完整显示单元数量，基于单调时间决定可显示的数量及各单元的淡入进度。一帧可以显示0、1或多个单元；不以“每帧一个字”计速。
2. **文本组件**：持有真正的文本、消息identity、Unicode切分、排版和阅读锚点，将引擎输出映射到字形透明度。数据到达、显示和淡入完成是三个不同的状态。

`typing::TypingMode::Arrivals`：同批到达的单元同批释放；`Paced`：按字速安排释放，不依赖输入chunk怎么分割或显示刷新率。`max_latency`可以限制突发积压的最大人工延迟，禁止在上游已停很久后仍慢慢拖出旧内容。空闲期间不攒字速额度，以免下一批突发直接倾倒。

视觉使用 smoothstep：`opacity = t²(3−2t)`，`t = clamp((当前逻辑时间−该单元释放时间)/淡入时长,0,1)`。暂停冻结逻辑时钟，因此文字数量和正在淡入的透明度一起暂停；恢复继续。Reduce Motion即时显现，恢复系统设置不会让旧文字重新淡入。切换消息应替换引擎与文本buffer，取消旧队列。

## Gallery

入口 Motion → Recipes：

- **定时打字与浮现**：已知完整文案按28个显示单元/s释放，默认180ms淡入。速度、淡入时长可调。
- **流式批量文字浮现**：模拟第一批1个单元，后续默认每批5个、间隔120ms到达。速度默认0，即按到达批量显示；调为正数切换到限速平滑，积压最多250ms。批量、间隔、字速、淡入可调。

A/B换文案，暂停、继续当前、重播和自动中途切换沿用现有控件。到达与显示计数分开，下面每一格代表一个完整单元的淡入状态。选择区五行，文档高度942pt。

这些默认值为TekesUI体验起点，不是Claude官方参数。示例是预分割完整单元，包含`👩‍💻`、`é`和`🇨🇳`。固定完整段落只排版一次，按UTF-16簇归属过滤字形，改变alpha；不为每一帧的新前缀重排文本，换行与基线保持稳定。

## 接入契约与边界

引擎不持有字符串、不读取系统时钟、不发起网络或帧调度，不依赖平台。调用方负责把流式UTF-8正确解码、按grapheme或shaped cluster切成**完整稳定单元**，跨chunk保留不完整的尾部单元，结束时再flush。不能把byte数、Unicode scalar数或模型token数直接当作“字数”。Gallery固定样本没有替任意生产流实现分段器。

```rust
use std::time::Duration;
use tekes_ui_motion::typing::{TypingConfig, TypingEngine};
let mut engine = TypingEngine::new(TypingConfig::default())?;
// 三个由调用方完成Unicode切分的显示单元到达。
engine.append(3, Duration::ZERO)?;
engine.advance(Duration::from_millis(100))?;
for unit in 0..engine.visible() {
    let alpha = engine.opacity(unit); // 用原排版位置绘制该单元的字形
}
# Ok::<(), tekes_ui_motion::MotionError>(())
```

每个实例最多65536个单元，超限返回错误，不静默丢字；长文档调用方应按文本块分段管理。已有完整单元的尾部修订、Markdown重排、选择/复制、阅读滚动、AX公告和真实LLM网络接线仍属于宿主文本组件；本轮未覆盖这些生产集成。真实原生视觉、混合脚本排版和整帧预算仍待统一验收。
