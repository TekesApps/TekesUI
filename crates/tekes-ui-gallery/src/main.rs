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
    if let Ok(slug) = std::env::var("TEKESUI_GALLERY_PAGE") {
        let page = gallery::Page::ALL
            .into_iter()
            .find(|p| p.slug() == slug)
            .unwrap_or_else(|| {
                eprintln!("Unknown Gallery page: {slug}");
                std::process::exit(2)
            });
        gallery_state.apply(gallery::Command::Page(page));
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
        Some("gallery") => selftest::run_script(selftest_gallery_script()),
        Some("text") => selftest::run_script(selftest_text_script()),
        Some("foundation") => selftest::run_script(selftest_foundation_script()),
        Some("components") => selftest::run_script(selftest_components_script()),
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
fn selftest_text_script() -> Vec<(u64, tekes_ui_platform_macos::selftest::Action)> {
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("text selftest output directory");
    let single = gallery::text_demo::TextDemo::editor_target(0);
    let multi = gallery::text_demo::TextDemo::editor_target(1);
    vec![
        (100, Action::MenuCommand(201)),
        (200, Action::MenuCommand(302)),
        (
            500,
            Action::MenuCommand(100 + gallery::Page::TextEditing.index() as u32),
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
        (9800, Action::MenuCommand(100)),
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
    use gallery::{Page, layout::PAGE_CONTENT, pages::motion};
    use tekes_ui_platform_macos::selftest::Action;
    let out = std::env::var("TEKES_SELFTEST_OUT").expect("foundation evidence directory");
    let capture = |name: &str| Action::Capture {
        path: format!("{out}/{name}.png"),
    };
    let page = |p: Page| Action::MenuCommand(100 + p.index() as u32);
    let click = |p: tekes_ui_scene::Point| Action::Click {
        x: f64::from(p.x),
        y: f64::from(p.y),
    };
    let motion = motion::Layout::new(PAGE_CONTENT);
    vec![
        (100, Action::MenuCommand(201)),
        (200, Action::MenuCommand(302)),
        (500, page(Page::Icons)),
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
        (
            3000,
            click(gallery::pages::icons::search_rect(PAGE_CONTENT).center()),
        ),
        (3200, Action::TypeAscii("SeArCh".into())),
        (
            3600,
            Action::ExpectText {
                text: "SeArCh".into(),
                marked: false,
            },
        ),
        (3800, capture("icons-search")),
        (4200, page(Page::TextEditing)),
        (
            4500,
            Action::Resize {
                w: 1440.0,
                h: 827.0,
            },
        ),
        (4900, click(gallery::text_demo::TextDemo::editor_target(0))),
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
        (7300, page(Page::Motion)),
        (
            7600,
            click(motion.button(motion::Action::Interrupt).center()),
        ),
        (7700, page(Page::Text)),
        (7900, page(Page::Motion)),
        (8300, capture("motion-resumed")),
        (
            8600,
            click(motion.button(motion::Action::ReduceMotion).center()),
        ),
        (8800, click(motion.button(motion::Action::Replay).center())),
        (9200, capture("motion-reduced")),
        (9500, Action::Key { key_code: 119 }),
        (9900, capture("motion-bottom")),
        (10200, Action::Resize { w: 720.0, h: 480.0 }),
        (10600, capture("motion-small")),
        (11000, Action::Log("idle interval starts".into())),
        (13500, Action::Close),
    ]
}
