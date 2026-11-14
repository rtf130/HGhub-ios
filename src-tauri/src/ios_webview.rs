//! iOS：全屏 WebView + 原生实测安全区 + 深色底色。
//!
//! 三件事，顺序固定：
//! 1. 从 `UIWindow.safeAreaInsets` 读出真实安全区（此时还没改 WebView 行为，值一定准），
//!    缓存到进程内，供前端 `safe_area_insets` 命令读取；
//! 2. 把 WKWebView 的 `scrollView.contentInsetAdjustmentBehavior` 设为 `.never`
//!    —— 页面铺满全屏，底部不再露出窗口底色（1.2.3 的效果）；
//! 3. 把 WKWebView / UIScrollView / UIWindow 底色刷黑，任何缝隙都不露白。
//!
//! 关键：`.never` 模式下 WebKit 会把 `env(safe-area-inset-*)` 报成 0，
//! 所以前端**不能**再用 env()，必须用这里缓存的实测值（见 App.tsx 的 MobileShell）。

use std::ffi::{c_char, c_void};
use std::sync::Mutex;

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

/// UIEdgeInsets：4 个 CGFloat（真机上 CGFloat = f64）
#[repr(C)]
struct UIEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

static INSETS: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));

struct Msg {
    msg0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    msg1_usize: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void,
    msg1_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
    msg1_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8),
    msg4: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, f64) -> *mut c_void,
    /// 返回 4×f64 的结构体，按 aarch64-apple 的 C ABI 走间接返回
    insets: unsafe extern "C" fn(*mut c_void, *mut c_void) -> UIEdgeInsets,
}

unsafe fn msgs() -> Msg {
    Msg {
        msg0: std::mem::transmute(objc_msgSend as *const ()),
        msg1_usize: std::mem::transmute(objc_msgSend as *const ()),
        msg1_ptr: std::mem::transmute(objc_msgSend as *const ()),
        msg1_bool: std::mem::transmute(objc_msgSend as *const ()),
        msg4: std::mem::transmute(objc_msgSend as *const ()),
        insets: std::mem::transmute(objc_msgSend as *const ()),
    }
}

unsafe fn sel(s: &[u8]) -> *mut c_void {
    sel_registerName(s.as_ptr() as *const c_char)
}

/// setup 里调用：量安全区 → 切全屏 → 刷底色
pub fn setup(app: &tauri::App) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let webview: &tauri::Webview<_> = window.as_ref();

    let _ = webview.with_webview(|pw| unsafe {
        let wk = pw.inner();
        if wk.is_null() {
            return;
        }
        let m = msgs();

        // ── 1. UIWindow.safeAreaInsets ──
        let ui_window = (m.msg0)(wk, sel(b"window\0"));
        if !ui_window.is_null() {
            let ins = (m.insets)(ui_window, sel(b"safeAreaInsets\0"));
            // 合法范围校验，异常值（ABI 不匹配时会拿到垃圾）一律丢弃
            let sane = |v: f64| v.is_finite() && (0.0..=200.0).contains(&v);
            if sane(ins.top) && sane(ins.bottom) {
                if let Ok(mut g) = INSETS.lock() {
                    *g = (ins.top, ins.bottom);
                }
            }
        }

        // ── 2. 全屏：关掉 UIKit 自动内缩（.never = 2）──
        let scroll = (m.msg0)(wk, sel(b"scrollView\0"));
        if !scroll.is_null() {
            (m.msg1_usize)(scroll, sel(b"setContentInsetAdjustmentBehavior:\0"), 2);
        }

        // ── 3. 底色刷黑（视频页上下留白、任何缝隙）──
        let color_cls = objc_getClass(b"UIColor\0".as_ptr() as *const c_char);
        if color_cls.is_null() {
            return;
        }
        let black = (m.msg4)(
            color_cls,
            sel(b"colorWithRed:green:blue:alpha:\0"),
            0.0,
            0.0,
            0.0,
            1.0,
        );
        if black.is_null() {
            return;
        }
        (m.msg1_ptr)(wk, sel(b"setBackgroundColor:\0"), black);
        (m.msg1_bool)(wk, sel(b"setOpaque:\0"), 0);
        if !scroll.is_null() {
            (m.msg1_ptr)(scroll, sel(b"setBackgroundColor:\0"), black);
        }
        if !ui_window.is_null() {
            (m.msg1_ptr)(ui_window, sel(b"setBackgroundColor:\0"), black);
        }
    });
}

/// 前端通过 `safe_area_insets` 命令读取（top, bottom），单位 pt
pub fn cached_insets() -> (f64, f64) {
    INSETS.lock().map(|g| *g).unwrap_or((0.0, 0.0))
}
