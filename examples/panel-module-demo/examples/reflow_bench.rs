use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
fn main() {
    let text = r###"拖动列与列之间的分隔线，观察这段文字如何随着可用宽度重新换行。窄列会产生更多行，宽列则能在同一行容纳更多内容。文字由当前模块进程独立排版和绘制，窗口只合成共享纹理。

Resize the panel to watch native text reflow. Words wrap at the available width; Chinese and English share the same paragraph. 每个模块保留自己的状态，调整列宽不会重启进程。

"###.repeat(8);
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let style = TextStyle::system(15.);
    let prepared = shaper.prepare(&text, &style);
    let mut fresh = Vec::new();
    let mut reused = Vec::new();
    let mut display = Vec::new();
    for i in 0..25 {
        let options = ShapeOptions {
            width: Some(180. + i as f64 * 7.),
            line_spacing: 5.,
        };
        let begin = std::time::Instant::now();
        let a = shaper.shape(&text, &style, &options);
        let elapsed = begin.elapsed().as_secs_f64() * 1000.;
        let begin = std::time::Instant::now();
        let b = shaper.shape_prepared(&prepared, &options);
        let reuse = begin.elapsed().as_secs_f64() * 1000.;
        let begin = std::time::Instant::now();
        let c = shaper.shape_prepared_display(&prepared, &options, 0.0..424.0);
        let painted = begin.elapsed().as_secs_f64() * 1000.;
        assert_eq!(a.lines.len(), c.line_count());
        assert_eq!(a.lines.len(), b.lines.len());
        if i > 0 {
            fresh.push(elapsed);
            reused.push(reuse);
            display.push(painted);
        }
    }
    display.sort_by(f64::total_cmp);
    println!(
        "viewport median {:.3} ms, p95 {:.3} ms",
        display[12], display[22]
    );
    fresh.sort_by(f64::total_cmp);
    reused.sort_by(f64::total_cmp);
    println!(
        "fresh median {:.3} ms, p95 {:.3} ms; prepared median {:.3} ms, p95 {:.3} ms",
        fresh[12], fresh[22], reused[12], reused[22]
    );
}
