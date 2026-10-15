//! 自测脚本支持，仅供示例和 Gallery 的自测模式使用，不属于边界 API。
//!
//! 后台线程按时间表向主队列投递步骤，步骤在主线程里直接操作 AppKit，
//! 与 osascript / 真实输入走同一批 AppKit 入口，不合成全局鼠标事件。
//! 只在自测模式下使用：空闲行为不受影响。

use std::ffi::c_void;
use std::time::Duration;

use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSSize};

unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, ctx: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// 一个自测步骤。
#[derive(Clone, Debug)]
pub enum Action {
    /// 改内容区尺寸，单位 pt。
    Resize {
        w: f64,
        h: f64,
    },
    Miniaturize,
    Deminiaturize,
    /// 用 `screencapture -o -l <窗口号> -x <path>` 截取窗口（设备像素，不含阴影）。
    /// 截图在单独的线程里做，不阻塞主线程。
    Capture {
        path: String,
    },
    /// 只打一行日志，用来在日志里标记阶段。
    Log(String),
    /// `performClose`：走 `CloseRequested` 路径。
    Close,
}

extern "C" fn run_step(ctx: *mut c_void) {
    // SAFETY: ctx 是 run_script 里 Box::into_raw 出来的 Action，每个只消费一次。
    let action = *unsafe { Box::from_raw(ctx.cast::<Action>()) };
    let mtm = MainThreadMarker::new().expect("主队列上必是主线程");
    let app = NSApplication::sharedApplication(mtm);
    let Some(w) = app.windows().firstObject() else {
        eprintln!("selftest: 没有窗口，跳过 {action:?}");
        return;
    };
    eprintln!("selftest: {action:?}");
    match action {
        Action::Resize { w: nw, h: nh } => w.setContentSize(NSSize::new(nw, nh)),
        Action::Miniaturize => w.miniaturize(None),
        Action::Deminiaturize => w.deminiaturize(None),
        Action::Log(_) => {}
        Action::Close => w.performClose(None),
        Action::Capture { path } => {
            let number = w.windowNumber();
            std::thread::spawn(move || {
                let status = std::process::Command::new("screencapture")
                    .args(["-o", "-x", "-l", &number.to_string(), &path])
                    .status();
                eprintln!("selftest: screencapture -> {path}: {status:?}");
            });
        }
    }
}

/// 在后台线程里按时间表（距调用时刻的毫秒数，需升序）执行步骤。
pub fn run_script(steps: Vec<(u64, Action)>) {
    std::thread::spawn(move || {
        let mut last = 0;
        for (t, action) in steps {
            std::thread::sleep(Duration::from_millis(t.saturating_sub(last)));
            last = t;
            let ctx = Box::into_raw(Box::new(action)).cast::<c_void>();
            // SAFETY: 主队列在进程生命周期内有效；run_step 在主线程消费 ctx。
            unsafe { dispatch_async_f(&raw const _dispatch_main_q, ctx, run_step) };
        }
    });
}
