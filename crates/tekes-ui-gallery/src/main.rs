//! Gallery 应用：恒定的人工目测验收目标，见章程第 2 节。
//!
//! 结构：
//! - `host.rs`：平台层与渲染层的接线（窗口事件 → `Gallery` → Renderer、窗口标题、菜单勾选）；
//! - `gallery/`：Gallery 自己的状态与内容（固定布局、导航、主题、页面、容器、快照），见 docs/gallery.md；
//! - 本文件：入口、窗口配置、命令行、自测模式。
//!
//! 命令行：
//! - 无参数：打开窗口；
//! - `--snapshot <目录>`：不开窗口，无头渲染每页 x 每个主题 x 缩放 2 和 1，写 PNG。
//!
//! 性能 harness：`TEKES_PERF=scroll[:秒数]` 开启，见 `perf.rs`；报告写到 `TEKES_PERF_OUT`。
//!
//! 自测模式（后台线程按时间表向主线程投递 AppKit 级动作，不合成全局输入事件）：
//! - `TEKES_SELFTEST=1`：缩放窗口、截图、最小化再恢复，见 docs/handoffs/F0-measurements.md；
//! - `TEKES_SELFTEST=gallery`：切页、切主题、hover、点击、方向键，每步截图，见 docs/gallery.md。
//! - `TEKES_SELFTEST=workbench-extended`：交互式组件 stories 的原生 AppKit smoke；不是像素差分或 VoiceOver 验收。
//!
//! 截图写到 `TEKES_SELFTEST_OUT` 指定的目录。

#[cfg(target_os = "macos")]
mod gallery;
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod perf;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Gallery 目前只支持 macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use tekes_ui_platform_macos::{App, AppConfig, Size, WindowConfig, WindowPlacement, selftest};

    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshot" => {
                let Some(dir) = args.next() else {
                    eprintln!("用法：tekes-ui-gallery --snapshot <输出目录>");
                    std::process::exit(2);
                };
                match gallery::snapshot::write_all(std::path::Path::new(&dir)) {
                    Ok(files) => {
                        for f in &files {
                            println!("{}", f.display());
                        }
                        eprintln!("共写出 {} 张", files.len());
                        return;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
            }
            other => {
                eprintln!("未知参数 {other}\n用法：tekes-ui-gallery [--snapshot <输出目录>]");
                std::process::exit(2);
            }
        }
    }

    let mode = std::env::var("TEKES_SELFTEST").ok();
    let perf_config = perf::PerfConfig::from_env();
    let mut gallery_state = gallery::Gallery::new();
    if let Ok(slug) = std::env::var("TEKESUI_GALLERY_PAGE")
        && !gallery_state.open_route(&slug)
    {
        eprintln!("Unknown Gallery route: {slug}");
        std::process::exit(2);
    }
    let menu_bar = gallery_state.menu_bar();
    let window_config = WindowConfig {
        title: gallery_state.title(),
        // 固定的内容区尺寸，Gallery 的每个页面都按它排版，保证可复现。
        size: Size {
            width: gallery::CONTENT_WIDTH as f64,
            height: gallery::CONTENT_HEIGHT as f64,
        },
        min_size: Some(Size {
            width: 720.0,
            height: 480.0,
        }),
        // 窗口摆放的开发规则：外接屏优先，否则内建屏右半边。缩放窗口的自测属于尺寸类检查。
        // 可用环境变量 TEKESUI_WINDOW_PLACEMENT 覆盖。
        placement: WindowPlacement::Auto {
            // 性能测量要 120Hz（只有内建 ProMotion 屏有），摆放规则会选内建屏。
            need_120hz: perf_config.is_some(),
            size_check: mode.as_deref() == Some("1"),
        },
        // 不保存窗口位置与尺寸：需要固定、可复现的尺寸。
        frame_autosave_name: None,
    };
    match mode.as_deref() {
        None => {}
        Some("workbench") => selftest::run_script(selftest_workbench_script()),
        Some("workbench-extended") => selftest::run_script(selftest_workbench_extended_script()),
        Some("gallery") => selftest::run_script(selftest_gallery_script()),
        Some("scroll-repair") => selftest::run_script(selftest_scroll_repair_script()),
        Some("reflow") => selftest::run_script(selftest_reflow_script()),
        Some("text") => selftest::run_script(selftest_text_script()),
        Some("foundation") => selftest::run_script(selftest_foundation_script()),
        Some("metal-lifecycle" | "metal-lifecycle-hold") => {
            selftest::run_script(selftest_metal_lifecycle_script())
        }
        Some("components") => selftest::run_script(selftest_components_script()),
        Some("motion-layout") => selftest::run_script(selftest_motion_layout_script()),
        Some(_) => selftest::run_script(selftest_script()),
    }
    App::new(AppConfig {
        name: Some("TekesUI Gallery".into()),
        ..AppConfig::default()
    })
    .run(
        window_config,
        menu_bar,
        host::Host::new(gallery_state).with_perf(perf_config.map(perf::Perf::new)),
    );
    if std::env::var_os("TEKES_PERF").is_some() {
        // 0：完整跑完且空闲确认通过；2：中途被关、没跑起来或空闲确认失败。
        std::process::exit(perf::exit_code());
    }
}

/// 自测时间表：初始截图；缩放到几个尺寸各截一张；最小化再恢复，截图确认内容还在。
#[cfg(target_os = "macos")]
fn selftest_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;

    let out = std::env::var("TEKES_SELFTEST_OUT").unwrap_or_else(|_| ".".into());
    let cap = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let sizes = [
        ("s1_800x600", 800.0, 600.0),
        ("s2_1000x520", 1000.0, 520.0),
        ("s3_1500x900", 1500.0, 900.0),
        ("s4_1200x800", 1200.0, 800.0),
    ];
    let mut steps = vec![(1500, cap("s0_initial_1200x800"))];
    let mut t = 2500;
    for (name, w, h) in sizes {
        steps.push((t, Action::Resize { w, h }));
        steps.push((t + 700, cap(name)));
        t += 1500;
    }
    steps.push((t, Action::Log("最小化".into())));
    steps.push((t + 100, Action::Miniaturize));
    steps.push((t + 2000, Action::Log("恢复".into())));
    steps.push((t + 2100, Action::Deminiaturize));
    steps.push((t + 3500, cap("s5_after_restore")));
    steps.push((t + 4500, Action::Close));
    steps
}

/// Gallery 自测时间表（`TEKES_SELFTEST=gallery`）。
///
/// 菜单命令编号见 `gallery/commands.rs`：页 100、101；外观 200（跟随）、201（浅）、202（深）；
/// 对比度 300（跟随）、301（开）、302（关）。日志里 `Redraw #n` 的行数用来核对哪一步重绘了。
/// 每一步之间留 700ms，让重绘完成再截图。
#[cfg(target_os = "macos")]
fn selftest_gallery_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action::{self, *};

    let out = std::env::var("TEKES_SELFTEST_OUT").unwrap_or_else(|_| ".".into());
    let cap = |name: &str| Capture {
        path: format!("{out}/{name}.png"),
    };
    let log = |s: &str| Log(s.into());
    // 目录第 0、1 行的中心（pt）：x 在列内，y 见 layout::catalog_row_rect。
    let row0 = (100.0, 32.0);
    let row1 = (100.0, 66.0);

    let mut steps: Vec<(u64, Action)> = Vec::new();
    let mut t = 1500;
    let mut at = |dt: u64, a: Action| {
        t += dt;
        steps.push((t, a));
    };
    // 先固定成浅色、标准对比度，使截图不依赖系统当前设置。
    at(0, WindowDark(false));
    at(0, MenuCommand(201));
    at(0, MenuCommand(302));
    at(700, cap("g01_primitives_light"));
    at(100, MenuCommand(101));
    at(700, cap("g02_tokens_light"));
    // 方向键：↑ 回到第一页，再 ↑ 到头不动。
    at(100, Key { key_code: 126 });
    at(700, cap("g03_primitives_after_up"));
    at(100, Key { key_code: 126 });
    at(100, log("到头后再按 ↑：不应有 Redraw"));
    // 点击目录第 1 行。
    at(
        500,
        Click {
            x: row1.0,
            y: row1.1,
        },
    );
    at(700, cap("g04_tokens_after_click"));
    // 主题：深、深+对比度、浅+对比度。
    at(100, MenuCommand(202));
    at(700, cap("g05_tokens_dark"));
    at(100, MenuCommand(301));
    at(700, cap("g06_tokens_dark_ic"));
    at(100, MenuCommand(201));
    at(700, cap("g07_tokens_light_ic"));
    at(100, MenuCommand(100));
    at(700, cap("g08_primitives_light_ic"));
    at(100, MenuCommand(202));
    at(700, cap("g09_primitives_dark_ic"));
    // 跟随系统：窗口外观设为深色 / 浅色，系统对比度保持原样（对比度项仍是“开”）。
    at(100, MenuCommand(200));
    at(100, MenuCommand(300));
    at(700, WindowDark(true));
    at(700, cap("g10_follow_system_dark"));
    at(100, WindowDark(false));
    at(700, cap("g11_follow_system_light"));
    // hover：进入第 0 行（一次重绘），行内移动三次（零次重绘），移出（一次重绘）。
    at(300, MenuCommand(100));
    at(500, log("hover 开始"));
    at(
        100,
        MouseMove {
            x: row0.0,
            y: row0.1,
        },
    );
    at(500, cap("g12_hover_row0"));
    at(
        100,
        MouseMove {
            x: row0.0 + 20.0,
            y: row0.1 + 2.0,
        },
    );
    at(
        100,
        MouseMove {
            x: row0.0 - 40.0,
            y: row0.1 - 3.0,
        },
    );
    at(
        100,
        MouseMove {
            x: row0.0,
            y: row0.1 + 5.0,
        },
    );
    at(500, log("行内移动结束：期间不应有 Redraw"));
    at(
        100,
        MouseMove {
            x: row1.0,
            y: row1.1,
        },
    );
    at(500, cap("g13_hover_row1"));
    at(100, MouseExit);
    at(500, log("hover 结束"));
    // 经 mainMenu.performKeyEquivalent 走快捷键匹配（⌃⌘ + 数字）：
    // 3 深、2 浅、5 对比度开、6 关、1 跟随、4 跟随。再用 ⌥⌘D（旧键）确认不再触发。
    let kq = |chars: &'static str, key_code: u16, control: bool, alt: bool, shift: bool| {
        Action::KeyEquivalent {
            chars,
            key_code,
            control,
            alt,
            shift,
            command: true,
        }
    };
    at(300, log("快捷键：⌃⌘3 深色"));
    at(100, kq("3", 20, true, false, false));
    at(700, cap("g14_key_dark"));
    at(100, log("快捷键：⌃⌘2 浅色"));
    at(100, kq("2", 19, true, false, false));
    at(700, cap("g15_key_light"));
    at(100, log("快捷键：⌃⌘5 对比度开"));
    at(100, kq("5", 23, true, false, false));
    at(700, cap("g16_key_ic_on"));
    at(100, kq("6", 22, true, false, false));
    at(100, kq("1", 18, true, false, false));
    at(100, kq("4", 21, true, false, false));
    at(100, log("旧键 ⌥⌘D：应 handled=false"));
    at(100, kq("d", 2, false, true, false));
    // 静态文字页（⌘3，命令 102）：固定浅色，选夹具与宽度（点击选择器按钮），再切深色。
    // 设了 TEKESUI_REFERENCE_DIR 时，参照窗格和差异窗格里是参照图和热图。
    let content = gallery::layout::PAGE_CONTENT;
    let btn = |r: tekes_ui_scene::Rect| {
        let c = r.center();
        (f64::from(c.x), f64::from(c.y))
    };
    let fixture_at = |i: usize| btn(gallery::pages::static_text::fixture_button(content, i));
    let width_at = |j: usize| btn(gallery::pages::static_text::width_button(content, j));
    at(300, MenuCommand(201));
    at(100, MenuCommand(302));
    at(100, MenuCommand(102));
    at(1500, cap("t01_text_sentences_a_light"));
    at(100, MenuCommand(202));
    at(1500, cap("t02_text_sentences_a_dark"));
    at(100, MenuCommand(201));
    let (x, y) = fixture_at(1);
    at(300, Click { x, y });
    at(1500, cap("t03_text_mixed_w700_light"));
    let (x, y) = width_at(1);
    at(100, Click { x, y });
    at(1500, cap("t04_text_mixed_w500_light"));
    let (x, y) = width_at(2);
    at(100, Click { x, y });
    at(1500, cap("t05_text_mixed_w320_light"));
    let (x, y) = fixture_at(3);
    at(100, Click { x, y });
    at(1500, cap("t06_text_emoji_w320_light"));
    let (x, y) = fixture_at(0);
    at(100, Click { x, y });
    at(1500, cap("t07_text_sentences_a_light_again"));
    at(100, Close);
    steps
}

/// Native-window smoke: real menu routing, state-chip clicks, screenshots, then close.
#[cfg(target_os = "macos")]
fn selftest_components_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use gallery::{Page, layout, pages::showcase};
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("components selftest output directory");
    let fixtures = [
        (Page::Foundations, "tui.split-view", "disabled"),
        (Page::Actions, "tui.button", "pressed"),
        (Page::Forms, "tui.text-field", "error"),
        (Page::Navigation, "tui.tree", "selected"),
        (Page::Overlays, "tui.dialog", "closing"),
        (Page::Feedback, "tui.status-badge", "error"),
        (Page::Documents, "tui.diff-view", "selection"),
        (Page::Data, "tui.chart", "selected"),
    ];
    let mut time = 600;
    let mut steps = vec![
        (100, Action::MenuCommand(201)),
        (200, Action::MenuCommand(302)),
    ];
    for (page, component, state) in fixtures {
        steps.push((time, Action::MenuCommand(100 + page.index() as u32)));
        time += 600;
        steps.push((
            time,
            Action::Capture {
                path: format!("{out}/{}-default.png", page.slug()),
            },
        ));
        time += 500;
        let point = showcase::state_center(
            page.category().unwrap(),
            layout::PAGE_CONTENT,
            component,
            state,
        )
        .expect("registered native state chip");
        steps.push((
            time,
            Action::Log(format!("showcase click {component}/{state}")),
        ));
        steps.push((
            time + 10,
            Action::Click {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
        ));
        time += 600;
        steps.push((
            time,
            Action::Capture {
                path: format!("{out}/{}-{state}.png", page.slug()),
            },
        ));
        time += 500;
        if page == Page::Foundations {
            steps.push((time, Action::Key { key_code: 121 }));
            time += 600;
            steps.push((
                time,
                Action::Capture {
                    path: format!("{out}/components-foundations-scrolled.png"),
                },
            ));
            time += 500;
            steps.push((time, Action::Key { key_code: 116 }));
            time += 200;
        }
    }
    steps.push((time, Action::Close));
    steps
}

#[cfg(target_os = "macos")]
fn selftest_reflow_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("reflow output directory");
    let mut steps = Vec::new();
    for (pass, (route, name)) in [
        ("component-text/playground", "component-text"),
        ("lab-internal-text/playground", "lab-internal-text"),
    ]
    .into_iter()
    .enumerate()
    {
        let base = 100 + pass as u64 * 6500;
        let page = gallery::workbench::route(route).expect("registered text story route");
        steps.push((
            base,
            Action::MenuCommand(gallery::commands::Command::Page(page).id().0),
        ));
        steps.push((
            base + 100,
            Action::Resize {
                w: 1440.0,
                h: 720.0,
            },
        ));
        for frame in 0..=240 {
            let progress = if frame <= 120 { frame } else { 240 - frame } as f64 / 120.0;
            steps.push((
                base + 700 + frame * 9,
                Action::Resize {
                    w: 1440.0 - 720.0 * progress,
                    h: 720.0,
                },
            ));
        }
        // Stable captures follow the continuous sweep; the sweep itself is not
        // a substitute for a presentation-time trace or physical live resize.
        for (offset, width, capture_name) in [
            (3400, 720.0, "narrow"),
            (4400, 1080.0, "medium"),
            (5400, 1440.0, "wide"),
        ] {
            steps.push((base + offset, Action::Resize { w: width, h: 720.0 }));
            steps.push((
                base + offset + 300,
                Action::Capture {
                    path: format!("{out}/{name}-{capture_name}.png"),
                },
            ));
        }
    }
    steps.push((13200, Action::Close));
    steps
}

#[cfg(target_os = "macos")]
fn selftest_scroll_repair_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("scroll-repair output");
    let story = |route: &str| {
        let page = gallery::workbench::route(route).expect("registered Gallery route");
        Action::MenuCommand(gallery::commands::Command::Page(page).id().0)
    };
    let capture = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let multi = gallery::Gallery::text_surface_center_for_selftest(
        "lab-internal-text/playground",
        1440.0,
        827.0,
        0,
        1,
    );
    let scroll_target = gallery::Gallery::text_surface_center_for_selftest(
        "component-scroll-area/playground",
        720.0,
        480.0,
        2,
        0,
    );
    let reading = if std::env::var("TEKESUI_GALLERY_LOCALE")
        .ok()
        .as_deref()
        .and_then(gallery::locale::Locale::from_code)
        == Some(gallery::locale::Locale::ZhCn)
    {
        include_str!("../../../fixtures/text/gallery-reading.txt")
    } else {
        include_str!("../../../fixtures/text/gallery-reading-en.txt")
    }
    .trim_end()
    .to_owned();
    vec![
        (100, story("lab-internal-text/playground")),
        (
            300,
            Action::Resize {
                w: 1440.0,
                h: 827.0,
            },
        ),
        (900, capture("editor-1440x827")),
        (
            1200,
            Action::Resize {
                w: 1000.0,
                h: 520.0,
            },
        ),
        (1800, capture("editor-1000x520")),
        (2100, Action::Resize { w: 720.0, h: 480.0 }),
        (2700, capture("editor-720x480")),
        (
            3000,
            Action::Resize {
                w: 1440.0,
                h: 827.0,
            },
        ),
        (
            3400,
            Action::Click {
                x: f64::from(multi.x),
                y: f64::from(multi.y),
            },
        ),
        (3600, Action::TextCommand("selectAll:")),
        (4000, capture("editor-selection")),
        (4300, Action::TextCommand("moveToEndOfDocument:")),
        (4800, capture("editor-last-line")),
        (5200, story("component-text/playground")),
        (5800, capture("reading-text")),
        (6200, story("component-scroll-area/playground")),
        (6800, capture("scroll-area-top")),
        // Preserve the original seven-page scenario coverage, then inspect the
        // canonical component stories with larger text and an explicit end query.
        (7600, story("component-scroll-area/playground")),
        (7800, Action::Resize { w: 720.0, h: 480.0 }),
        (
            8000,
            Action::MenuCommand(gallery::commands::Command::TextScale(2).id().0),
        ),
        (8300, capture("scroll-area-720x480-150-top")),
        (
            8400,
            Action::Click {
                x: f64::from(scroll_target.x),
                y: f64::from(scroll_target.y),
            },
        ),
        (
            8500,
            Action::ExpectText {
                text: reading.clone(),
                marked: false,
            },
        ),
        (8600, Action::TextCommand("moveToEndOfDocument:")),
        (
            8750,
            Action::ExpectText {
                text: reading,
                marked: false,
            },
        ),
        (8900, capture("scroll-area-720x480-150-bottom")),
        (9200, story("component-text-area/playground")),
        (9400, capture("text-area-720x480-150")),
        // PageDown reveals the middle TextArea field; End would move past it
        // to the event log at the bottom of the preview.
        (9550, Action::Key { key_code: 121 }),
        (9800, capture("text-area-720x480-150-scrolled")),
        (10000, story("component-text-field/playground")),
        (
            10200,
            Action::Resize {
                w: 1000.0,
                h: 720.0,
            },
        ),
        (10500, capture("text-field-1000x720-150")),
        (10800, story("pattern-master/playground")),
        (11000, Action::Resize { w: 720.0, h: 480.0 }),
        (11300, capture("master-detail-720x480-150")),
        (11700, Action::Close),
    ]
}

#[cfg(target_os = "macos")]
fn selftest_text_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("text selftest output directory");
    let route = "lab-internal-text/playground";
    let single = gallery::Gallery::text_surface_center_for_selftest(route, 1440.0, 827.0, 0, 0);
    let multi = gallery::Gallery::text_surface_center_for_selftest(route, 1440.0, 827.0, 0, 1);
    let page = gallery::workbench::route(route).expect("registered internal TextEditor story");
    vec![
        (100, Action::MenuCommand(201)),
        (200, Action::MenuCommand(302)),
        (
            400,
            Action::Resize {
                w: 1440.0,
                h: 827.0,
            },
        ),
        (
            500,
            Action::MenuCommand(gallery::commands::Command::Page(page).id().0),
        ),
        (
            1100,
            Action::Capture {
                path: format!("{out}/text-default.png"),
            },
        ),
        (
            1700,
            Action::Click {
                x: f64::from(single.x),
                y: f64::from(single.y),
            },
        ),
        (1900, Action::TextCommand("selectAll:")),
        (2100, Action::TypeAscii("Hello ".into())),
        (2400, Action::Preedit("qing".into())),
        (
            2700,
            Action::ExpectText {
                text: "Hello qing".into(),
                marked: true,
            },
        ),
        (
            2900,
            Action::Capture {
                path: format!("{out}/text-preedit.png"),
            },
        ),
        (3500, Action::Text("清晰👨‍👩‍👧‍👦e\u{301}".into())),
        (
            3800,
            Action::ExpectText {
                text: "Hello 清晰👨‍👩‍👧‍👦e\u{301}".into(),
                marked: false,
            },
        ),
        (4000, Action::TextCommand("deleteBackward:")),
        (4200, Action::TextCommand("deleteBackward:")),
        (
            4400,
            Action::ExpectText {
                text: "Hello 清晰".into(),
                marked: false,
            },
        ),
        (4600, Action::TextCommand("undo:")),
        (4800, Action::TextCommand("undo:")),
        (
            5000,
            Action::ExpectText {
                text: "Hello 清晰👨‍👩‍👧‍👦e\u{301}".into(),
                marked: false,
            },
        ),
        (5200, Action::TextCommand("selectAll:")),
        (5400, Action::TextCommand("copy:")),
        (5600, Action::TextCommand("cut:")),
        (5800, Action::TextCommand("paste:")),
        (
            6000,
            Action::ExpectText {
                text: "Hello 清晰👨‍👩‍👧‍👦e\u{301}".into(),
                marked: false,
            },
        ),
        (
            6200,
            Action::Capture {
                path: format!("{out}/text-committed.png"),
            },
        ),
        (
            6800,
            Action::Click {
                x: f64::from(multi.x),
                y: f64::from(multi.y),
            },
        ),
        (7000, Action::TextCommand("selectAll:")),
        (7200, Action::Text("第一行\n第二行👨‍👩‍👧‍👦\n第三行".into())),
        (
            7500,
            Action::ExpectText {
                text: "第一行\n第二行👨‍👩‍👧‍👦\n第三行".into(),
                marked: false,
            },
        ),
        (7700, Action::TextCommand("undo:")),
        (7900, Action::TextCommand("moveToEndOfDocument:")),
        (
            8300,
            Action::Capture {
                path: format!("{out}/text-last-line.png"),
            },
        ),
        (8900, Action::TextCommand("moveToBeginningOfDocument:")),
        (
            9200,
            Action::Capture {
                path: format!("{out}/text-first-line.png"),
            },
        ),
        (
            9800,
            Action::MenuCommand(
                gallery::commands::Command::Page(
                    gallery::workbench::route("lab-calibration/playground")
                        .expect("registered calibration route"),
                )
                .id()
                .0,
            ),
        ),
        (10000, Action::Key { key_code: 121 }),
        (
            10300,
            Action::Capture {
                path: format!("{out}/primitives-scrolled.png"),
            },
        ),
        (11000, Action::Close),
    ]
}

/// Integrated native smoke test. Physical gestures/IME still require manual acceptance.
#[cfg(target_os = "macos")]
fn selftest_foundation_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use gallery::{motion_preview::PresetCatalog, pages::motion::Action as MotionAction};
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("foundation evidence directory");
    let capture = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let page = |route: &str| {
        let page = gallery::workbench::route(route).expect("registered Foundation story route");
        Action::MenuCommand(gallery::commands::Command::Page(page).id().0)
    };
    let click = |p: tekes_ui_scene::Point| Action::Click {
        x: f64::from(p.x),
        y: f64::from(p.y),
    };
    let icon_search = gallery::Gallery::text_surface_center_for_selftest(
        "component-icon/playground",
        1000.0,
        520.0,
        0,
        0,
    );
    let editor_target = gallery::Gallery::text_surface_center_for_selftest(
        "lab-internal-text/playground",
        1440.0,
        827.0,
        0,
        0,
    );
    let motion_interrupt = gallery::Gallery::motion_action_center_for_selftest(
        1440.0,
        827.0,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Interrupt,
    );
    let motion_reduce = gallery::Gallery::motion_action_center_for_selftest(
        1440.0,
        827.0,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::ReduceMotion,
    );
    let motion_replay = gallery::Gallery::motion_action_center_for_selftest(
        1440.0,
        827.0,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Replay,
    );
    vec![
        (100, Action::MenuCommand(201)),
        (200, Action::MenuCommand(302)),
        (500, page("component-icon/playground")),
        (1000, capture("icons-top")),
        (1300, Action::Key { key_code: 119 }),
        (1700, capture("icons-bottom")),
        (
            2000,
            Action::Resize {
                w: 1000.0,
                h: 520.0,
            },
        ),
        (2400, capture("icons-resized-bottom")),
        (2700, Action::Key { key_code: 115 }),
        (3000, click(icon_search)),
        (3200, Action::TypeAscii("SeArCh".into())),
        (
            3600,
            Action::ExpectText {
                text: "SeArCh".into(),
                marked: false,
            },
        ),
        (3800, capture("icons-search")),
        (4200, page("lab-internal-text/playground")),
        (
            4500,
            Action::Resize {
                w: 1440.0,
                h: 827.0,
            },
        ),
        (4900, click(editor_target)),
        (5100, Action::TextCommand("selectAll:")),
        (5300, Action::TypeAscii("Focus preserved".into())),
        (5600, Action::Miniaturize),
        (6200, Action::Deminiaturize),
        (
            6600,
            Action::ExpectText {
                text: "Focus preserved".into(),
                marked: false,
            },
        ),
        (6800, capture("text-focus-restored")),
        (7300, page("foundation-motion/playground")),
        (7600, click(motion_interrupt)),
        (7700, page("component-text/playground")),
        (7900, page("foundation-motion/playground")),
        (8300, capture("motion-resumed")),
        (8600, click(motion_reduce)),
        (8800, click(motion_replay)),
        (9200, capture("motion-reduced")),
        (9500, Action::Key { key_code: 119 }),
        (9900, capture("motion-bottom")),
        (10200, Action::Resize { w: 720.0, h: 480.0 }),
        (10600, capture("motion-small")),
        (11000, Action::Log("idle interval starts".into())),
        (13500, Action::Close),
    ]
}

/// Regresses the canonical Motion story's live-preview origin through actual
/// mouse-move and click events after preview text scaling, resize, and scroll.
#[cfg(target_os = "macos")]
fn selftest_motion_layout_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use gallery::{
        Command, locale::Locale, motion_preview::PresetCatalog,
        pages::motion::Action as MotionAction,
    };
    use tekes_ui_platform_macos::selftest::Action;

    let out = std::env::var("TEKES_SELFTEST_OUT").expect("motion layout evidence directory");
    std::fs::create_dir_all(&out).expect("create motion layout evidence directory");
    let capture = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let click = |p: tekes_ui_scene::Point| Action::Click {
        x: f64::from(p.x),
        y: f64::from(p.y),
    };
    let move_to = |p: tekes_ui_scene::Point| Action::MouseMove {
        x: f64::from(p.x),
        y: f64::from(p.y),
    };
    let page = gallery::workbench::route("foundation-motion/playground")
        .expect("canonical Motion playground route");
    let recipe_button = gallery::Gallery::motion_action_center_for_selftest(
        gallery::CONTENT_WIDTH,
        gallery::CONTENT_HEIGHT,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Catalog(PresetCatalog::Recipes),
    );
    let symbols_button = gallery::Gallery::motion_action_center_for_selftest(
        gallery::CONTENT_WIDTH,
        gallery::CONTENT_HEIGHT,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Catalog(PresetCatalog::Symbols),
    );
    let symbols_linear_rotation = gallery::Gallery::motion_action_center_for_selftest(
        gallery::CONTENT_WIDTH,
        gallery::CONTENT_HEIGHT,
        0,
        false,
        PresetCatalog::Symbols,
        MotionAction::Preset(tekes_ui_motion::symbols::SymbolEffect::ALL.len()),
    );
    let design_system_button = gallery::Gallery::motion_action_center_for_selftest(
        gallery::CONTENT_WIDTH,
        gallery::CONTENT_HEIGHT,
        0,
        false,
        PresetCatalog::Symbols,
        MotionAction::Catalog(PresetCatalog::DesignSystem),
    );
    let target_expand = gallery::Gallery::motion_action_center_for_selftest(
        1440.0,
        960.0,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Target(2),
    );
    let target_reverse = gallery::Gallery::motion_action_center_for_selftest(
        1440.0,
        960.0,
        0,
        false,
        PresetCatalog::Recipes,
        MotionAction::Target(0),
    );
    let target_reverse_click = click(target_reverse);
    let calibration_page = gallery::workbench::route("lab-calibration/playground")
        .expect("canonical calibration playground route");
    let calibration_fixture =
        gallery::Gallery::calibration_selector_center_for_selftest(1000.0, 520.0, 1, 1, false);
    let calibration_width =
        gallery::Gallery::calibration_selector_center_for_selftest(1000.0, 520.0, 1, 1, true);
    vec![
        (100, Action::MenuCommand(Command::Page(page).id().0)),
        (700, capture("motion-layout-initial")),
        (1000, move_to(recipe_button)),
        (1300, capture("motion-layout-hover-preview-origin")),
        (1600, click(recipe_button)),
        (1900, capture("motion-layout-catalog-click")),
        (2100, click(symbols_button)),
        (2400, move_to(symbols_linear_rotation)),
        (2700, capture("motion-layout-symbols-linear-rotation-hover")),
        (3000, click(symbols_linear_rotation)),
        (3300, capture("motion-layout-symbols-linear-rotation")),
        (3500, click(design_system_button)),
        (3800, capture("motion-layout-design-system")),
        (4000, click(recipe_button)),
        (4300, capture("motion-layout-recipes-restored")),
        (
            4600,
            Action::Resize {
                w: 1440.0,
                h: 960.0,
            },
        ),
        (5000, Action::MenuCommand(Command::TextScale(0).id().0)),
        // Match fixture_point's unscrolled rendered Button geometry before native target clicks.
        (5400, Action::Key { key_code: 115 }),
        (5900, move_to(target_reverse)),
        (6300, capture("motion-layout-target-hover")),
        (6700, target_reverse_click.clone()),
        (7100, capture("motion-layout-target-collapsed")),
        (7400, move_to(target_expand)),
        (7700, click(target_expand)),
        (8100, capture("motion-layout-target-expanded")),
        (8400, target_reverse_click),
        (8800, capture("motion-layout-target-reversed")),
        (9000, Action::Key { key_code: 119 }),
        (9300, capture("motion-layout-scroll-bottom")),
        (9600, Action::Key { key_code: 115 }),
        (9900, capture("motion-layout-scroll-top")),
        (10200, Action::Resize { w: 800.0, h: 600.0 }),
        (10500, Action::MenuCommand(Command::TextScale(1).id().0)),
        (
            10800,
            Action::MenuCommand(Command::Language(Locale::En).id().0),
        ),
        (11100, Action::Key { key_code: 115 }),
        (11400, capture("motion-layout-800-en-125-top")),
        (11700, Action::Key { key_code: 119 }),
        (12000, capture("motion-layout-800-en-125-bottom")),
        (12300, Action::Key { key_code: 115 }),
        (12600, Action::Resize { w: 720.0, h: 480.0 }),
        (12900, Action::MenuCommand(Command::TextScale(2).id().0)),
        (13200, capture("motion-layout-720-en-150-top")),
        (13500, Action::Key { key_code: 119 }),
        (13800, capture("motion-layout-720-en-150-bottom")),
        (14100, Action::Key { key_code: 115 }),
        (
            14400,
            Action::MenuCommand(Command::Language(Locale::ZhCn).id().0),
        ),
        (14700, capture("motion-layout-720-zh-150-top")),
        (15000, Action::Key { key_code: 119 }),
        (15300, capture("motion-layout-720-zh-150-bottom")),
        (15600, Action::Key { key_code: 115 }),
        (15900, Action::Resize { w: 800.0, h: 600.0 }),
        (16200, Action::MenuCommand(Command::TextScale(1).id().0)),
        (16500, capture("motion-layout-800-zh-125-top")),
        (16800, Action::Key { key_code: 119 }),
        (17100, capture("motion-layout-800-zh-125-bottom")),
        (
            17400,
            Action::Resize {
                w: 1000.0,
                h: 520.0,
            },
        ),
        (
            17700,
            Action::MenuCommand(Command::Page(calibration_page).id().0),
        ),
        (
            18100,
            Action::MenuCommand(Command::Language(Locale::En).id().0),
        ),
        (18400, Action::MenuCommand(Command::TextScale(2).id().0)),
        (18800, Action::MenuCommand(Command::PreviewWidth(1).id().0)),
        (19200, capture("motion-layout-calibration-large-type")),
        (19600, click(calibration_fixture)),
        (20000, capture("motion-layout-calibration-fixture-selected")),
        (20400, click(calibration_width)),
        (20800, capture("motion-layout-calibration-width-selected")),
        (21300, Action::Close),
    ]
}

/// Native rendering regression, not a performance run or physical resize capture.
fn selftest_metal_lifecycle_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use gallery::{layout::PAGE_CONTENT, motion_preview::PresetCatalog, pages::motion};
    use tekes_ui_platform_macos::selftest::Action;
    let layout = motion::Layout::new(PAGE_CONTENT);
    let click = |action| {
        let mut rect = layout.button(action);
        if matches!(
            action,
            motion::Action::Target(_)
                | motion::Action::Resume
                | motion::Action::Replay
                | motion::Action::Interrupt
        ) {
            rect.origin.y = PAGE_CONTENT.min_y() + 238.0;
            rect.size.height = 26.0;
        }
        let point = rect.center();
        Action::Click {
            x: point.x as f64,
            y: point.y as f64,
        }
    };
    let active = |label| Action::FrameExpect {
        label,
        min: 10,
        max: 300,
    };
    let idle = |label| Action::FrameExpect {
        label,
        min: 0,
        max: 0,
    };
    let mut steps = vec![
        (100, Action::FrameMark("observe")),
        (300, click(motion::Action::Catalog(PresetCatalog::Symbols))),
        (500, click(motion::Action::Preset(14))),
        (1000, Action::FrameMark("linear-start")),
        (1800, active("initial-playback")),
    ];
    let mut base = 2000;
    for _ in 0..3 {
        steps.extend([
            (base, Action::BeginLiveResize),
            (
                base + 100,
                Action::Resize {
                    w: 1200.0,
                    h: 760.0,
                },
            ),
            (
                base + 200,
                Action::Resize {
                    w: 1000.0,
                    h: 680.0,
                },
            ),
            (base + 400, Action::FrameMark("during-live-resize")),
            (base + 1000, active("during-live-resize-keeps-presenting")),
            (base + 1100, Action::EndLiveResize),
            (base + 1300, Action::FrameMark("after-resize")),
            (base + 2100, active("resize-resumed")),
            (base + 2200, click(motion::Action::Target(0))),
            (base + 2800, active("retarget-reverse")),
            (base + 2900, click(motion::Action::Target(1))),
            (base + 3300, Action::FrameMark("paused-settled")),
            (base + 3900, idle("paused-no-presents")),
            (base + 4000, click(motion::Action::Resume)),
            (base + 4800, active("pause-resume")),
            (
                base + 4900,
                Action::Resize {
                    w: 1440.0,
                    h: 827.0,
                },
            ),
            (base + 5300, Action::FrameMark("after-programmatic-resize")),
            (base + 6100, active("programmatic-resize-resumed")),
            (base + 6200, click(motion::Action::Target(2))),
            (base + 6800, active("retarget-expand")),
        ]);
        base += 7000;
    }
    if std::env::var("TEKES_SELFTEST").as_deref() != Ok("metal-lifecycle-hold") {
        steps.push((base, Action::Close));
    }
    steps
}

#[cfg(target_os = "macos")]
fn selftest_workbench_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("workbench evidence directory");
    let mut steps = Vec::new();
    let mut time = 500;
    let mut add = |action| {
        time += 450;
        steps.push((time, action));
    };
    let page = |route: &str| {
        Action::MenuCommand(
            gallery::Command::Page(gallery::workbench::route(route).unwrap())
                .id()
                .0,
        )
    };
    let click = |route: &str, id| {
        let p = gallery::workbench::Workbench::fixture_point(route, id);
        Action::Click {
            x: p.x as f64,
            y: p.y as f64,
        }
    };
    let cap = |name| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    add(Action::MenuCommand(201));
    add(Action::MenuCommand(302));
    add(page("example-settings"));
    add(cap("01-settings-initial"));
    add(click("example-settings", 1000));
    add(Action::TextCommand("selectAll:"));
    add(Action::Preedit("中".into()));
    add(Action::ExpectText {
        text: "中".into(),
        marked: true,
    });
    add(Action::Text("Project Studio".into()));
    add(Action::ExpectText {
        text: "Project Studio".into(),
        marked: false,
    });
    add(click("example-settings", 20));
    add(cap("02-settings-pending"));
    add(click("example-settings", 21));
    add(cap("03-settings-failed"));
    add(click("example-settings", 20));
    add(click("example-settings", 21));
    add(cap("04-settings-saved"));
    add(page("component-menu"));
    add(click("component-menu", 20));
    add(Action::Key { key_code: 125 });
    add(cap("05-menu-keyboard"));
    add(Action::Key { key_code: 36 });
    add(cap("06-menu-action"));
    add(page("component-dialog"));
    add(click("component-dialog", 20));
    add(Action::Key { key_code: 48 });
    add(cap("07-dialog-focus"));
    add(Action::Key { key_code: 53 });
    add(cap("08-dialog-restored"));
    add(Action::MenuCommand(521));
    add(page("component-button"));
    add(cap("09-button-blue"));
    add(Action::MenuCommand(202));
    add(cap("10-button-dark"));
    add(Action::MenuCommand(542));
    add(Action::MenuCommand(501));
    add(cap("11-large-text-narrow"));
    add(Action::MenuCommand(540));
    add(Action::MenuCommand(500));
    add(Action::MenuCommand(520));
    add(Action::MenuCommand(201));
    add(page("pattern-master"));
    add(cap("12-master-detail"));
    add(Action::Resize { w: 800., h: 600. });
    add(cap("13-master-narrow"));
    add(page("component-scroll-area"));
    add(cap("14-scroll-viewport"));
    add(Action::Resize { w: 1440., h: 960. });
    add(page("foundation-tokens"));
    add(cap("15-foundation-colors"));
    let search = gallery::navigation::search_rect().center();
    add(Action::Click {
        x: search.x as f64,
        y: search.y as f64,
    });
    add(cap("16-navigation-search-focus"));
    add(Action::Text("component-button/states".into()));
    add(Action::Resize { w: 800., h: 600. });
    add(cap("17-navigation-search-resized"));
    add(Action::TextCommand("selectAll:"));
    add(Action::TextCommand("deleteBackward:"));
    add(Action::Resize { w: 1440., h: 960. });
    add(page("foundation-shape"));
    add(cap("18-foundation-shape"));
    add(page("foundation-accessibility"));
    add(cap("19-foundation-interaction"));
    add(page("lab-geometry"));
    add(cap("20-raster-calibration"));
    add(Action::Close);
    steps
}

/// Native AppKit smoke across the production extended-control stories.
/// This exercises their real hit targets and keyboard paths; it is not an
/// automated pixel-diff or VoiceOver acceptance run.
#[cfg(target_os = "macos")]
fn selftest_workbench_extended_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;

    let out = std::env::var("TEKES_SELFTEST_OUT").expect("extended workbench evidence directory");
    std::fs::create_dir_all(&out).expect("create extended workbench evidence directory");
    let mut steps = Vec::new();
    let mut time = 500;
    let mut add = |action| {
        time += 500;
        steps.push((time, action));
    };
    let page = |route: &str| {
        Action::MenuCommand(
            gallery::Command::Page(gallery::workbench::route(route).unwrap())
                .id()
                .0,
        )
    };
    let click = |route: &str, id| {
        let point = gallery::workbench::Workbench::fixture_point(route, id);
        Action::Click {
            x: point.x as f64,
            y: point.y as f64,
        }
    };
    let capture = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };

    // Fix theme and contrast so the evidence is reproducible.
    add(Action::MenuCommand(201));
    add(Action::MenuCommand(302));

    let route = "component-switch";
    add(page(route));
    add(capture("x01-switch-initial"));
    add(click(route, 20));
    add(capture("x02-switch-pointer-toggle"));
    add(Action::Key { key_code: 49 }); // Space
    add(capture("x03-switch-space-toggle"));

    let route = "component-radio-group";
    add(page(route));
    add(capture("x04-radio-initial"));
    add(click(route, 31));
    add(capture("x05-radio-pointer-select"));
    add(Action::Key { key_code: 124 }); // Right
    add(capture("x06-radio-arrow-select"));

    let route = "component-slider";
    add(page(route));
    add(capture("x07-slider-initial"));
    add(click(route, 20));
    add(Action::Key { key_code: 124 }); // Right, increment by the model step
    add(capture("x08-slider-key-right"));
    add(Action::Key { key_code: 115 }); // Home
    add(capture("x09-slider-home"));
    add(Action::Key { key_code: 119 }); // End
    add(capture("x10-slider-end"));

    let route = "component-disclosure";
    add(page(route));
    add(capture("x11-disclosure-initial"));
    add(click(route, 20));
    add(capture("x12-disclosure-pointer-toggle"));
    add(Action::Key { key_code: 36 }); // Return
    add(capture("x13-disclosure-enter-toggle"));

    let route = "component-tree";
    add(page(route));
    add(capture("x14-tree-initial"));
    add(click(route, 30));
    add(Action::Key { key_code: 125 }); // Down through visible rows
    add(capture("x15-tree-down"));
    add(Action::Key { key_code: 124 }); // Right expands or enters a child
    add(capture("x16-tree-right"));

    let route = "component-data-table";
    add(page(route));
    add(capture("x17-data-table-initial"));
    add(click(route, 20)); // Sort by the first column
    add(capture("x18-data-table-sorted"));
    add(Action::Key { key_code: 125 });
    add(Action::Key { key_code: 36 }); // Return selects the focused row
    add(capture("x19-data-table-row-selected"));

    let route = "component-combo-box";
    add(page(route));
    add(capture("x20-combo-initial"));
    add(click(route, 1000)); // The actual native text-input fixture
    add(Action::Preedit("C".into()));
    add(Action::ExpectText {
        text: "C".into(),
        marked: true,
    });
    add(Action::Text("Com".into()));
    add(Action::ExpectText {
        text: "Com".into(),
        marked: false,
    });
    add(capture("x21-combo-filtered"));
    add(Action::TextCommand("moveDown:")); // AppKit doCommandBySelector for Down
    add(Action::TextCommand("insertNewline:")); // AppKit doCommandBySelector for Return
    add(Action::ExpectText {
        text: "Comfortable".into(),
        marked: false,
    });
    add(capture("x22-combo-enter-committed"));
    add(Action::Key { key_code: 125 }); // Reopen from the input
    add(Action::Key { key_code: 53 }); // Escape closes without a wall-clock wait
    add(capture("x23-combo-escape"));
    add(click(route, 1000));
    add(Action::TextCommand("selectAll:"));
    add(Action::TextCommand("deleteBackward:")); // Restore the all-options fixture
    add(capture("x24-combo-options-restored"));
    add(click(route, 42)); // Pointer-select the third real option
    add(Action::ExpectText {
        text: "Comfortable".into(),
        marked: false,
    });
    add(capture("x25-combo-pointer-option"));

    let route = "component-popover";
    add(page(route));
    add(capture("x26-popover-initial"));
    add(click(route, 20));
    add(capture("x27-popover-open"));
    add(Action::Key { key_code: 53 }); // Escape restores invoker focus
    add(capture("x28-popover-escape-restored"));

    let route = "component-toast";
    add(page(route));
    add(capture("x29-toast-initial"));
    add(click(route, 20));
    add(capture("x30-toast-dismissed"));
    add(click(route, 21)); // Restore deterministic visible fixture
    add(capture("x31-toast-restored"));

    let route = "component-chart";
    add(page(route));
    add(capture("x32-chart-initial"));
    add(click(route, 32)); // Select the third data-driven bar
    add(capture("x33-chart-pointer-selected"));
    add(Action::Key { key_code: 124 }); // Move to the next finite data point
    add(capture("x34-chart-arrow-selected"));

    add(Action::Close);
    steps
}
