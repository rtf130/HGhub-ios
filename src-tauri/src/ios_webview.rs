//! iOS：WKWebView 全屏 + 深色底色。
//!
//! 为什么分两步：
//! * `contentInsetAdjustmentBehavior = .never` 能让页面铺满全屏（不再露出窗口底色），
//!   但 WebKit 在这种模式下会把 `env(safe-area-inset-*)` 报成 0 —— 页面的 `--sat`/`--sab`
//!   会掉到兜底值，顶部内容钻进刘海、返回键被状态栏挡住。
//! * 所以顺序是：前端先用不可见探针在 `.automatic` 状态下量出真实安全区，
//!   写进 CSS 变量 `--sat`/`--sab`，**然后**再调 `ios_fullbleed` 把原生切成全屏。
//!   之后页面用实测值排版：既铺满全屏，又不被刘海/Home 条遮挡。

use std::ffi::{c_char, c_void};

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

struct Msg {
    msg0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    msg1_usize: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void,
    msg1_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
    msg1_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8),
    msg4: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, f64) -> *mut c_void,
}

unsafe fn msgs() -> Msg {
    Msg {
        msg0: std::mem::transmute(objc_msgSend as *const ()),
        msg1_usize: std::mem::transmute(objc_msgSend as *const ()),
        msg1_ptr: std::mem::transmute(objc_msgSend as *const ()),
        msg1_bool: std::mem::transmute(objc_msgSend as *const ()),
        msg4: std::mem::transmute(objc_msgSend as *const ()),
    }
}

unsafe fn sel(s: &[u8]) -> *mut c_void {
    sel_registerName(s.as_ptr() as *const c_char)
}

/// 把 WKWebView / UIScrollView / UIWindow 底色刷成纯黑，避免任何缝隙露出系统白底。
pub fn paint_dark_background(app: &tauri::App) {
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

        let scroll = (m.msg0)(wk, sel(b"scrollView\0"));
        if !scroll.is_null() {
            (m.msg1_ptr)(scroll, sel(b"setBackgroundColor:\0"), black);
        }

        let ui_window = (m.msg0)(wk, sel(b"window\0"));
        if !ui_window.is_null() {
            (m.msg1_ptr)(ui_window, sel(b"setBackgroundColor:\0"), black);
        }
    });
}

/// 前端量完安全区后调用：关掉 UIKit 的自动内缩，让页面真正铺满全屏。
pub fn apply_fullbleed(app: &tauri::AppHandle) {
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

        let scroll = (m.msg0)(wk, sel(b"scrollView\0"));
        if scroll.is_null() {
            return;
        }
        // UIScrollViewContentInsetAdjustmentBehaviorNever = 2
        (m.msg1_usize)(scroll, sel(b"setContentInsetAdjustmentBehavior:\0"), 2);
    });
}
