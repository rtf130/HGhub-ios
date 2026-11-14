//! iOS：WKWebView 底色修正。
//!
//! 背景：UIKit 默认 `contentInsetAdjustmentBehavior = automatic`，会按安全区给页面套一层
//! content inset；而项目 CSS 里 MobileShell 已经用 `--sat` / `--sab` 处理过一次安全区，
//! 视觉上就是「底栏悬空、下面露出窗口底色」。
//!
//! 试过把它设成 `.never`（铺满全屏）：白条是没了，但 WebKit 在这种情况下会把
//! `env(safe-area-inset-*)` 报成 0，页面的 `--sat/--sab` 直接掉到兜底值（12/16px），
//! 顶部内容钻进刘海（返回键被状态栏挡住）、整体高度变化导致画面像被放大 —— 收益不划算。
//!
//! 所以这里保留系统内缩（布局正确），只把 WKWebView / UIScrollView / UIWindow 的底色
//! 刷成纯黑：那条露出来的带子从白变黑，和深色界面、视频页上下留白连成一片，不再割裂。

use std::ffi::{c_char, c_void};

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

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

        let msg0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg1_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg1_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg4: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            f64,
            f64,
            f64,
            f64,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());

        let sel = |s: &[u8]| sel_registerName(s.as_ptr() as *const c_char);

        // 纯黑（#000）：视频页上下留白、底栏下方露出的带子都用这个色，视觉上连成一片
        let color_cls = objc_getClass(b"UIColor\0".as_ptr() as *const c_char);
        if color_cls.is_null() {
            return;
        }
        let black = msg4(
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

        // 1. WebView 本体
        msg1_ptr(wk, sel(b"setBackgroundColor:\0"), black);
        msg1_bool(wk, sel(b"setOpaque:\0"), 0);

        // 2. 内部 UIScrollView（越界/弹跳区域也会露出来）
        let scroll = msg0(wk, sel(b"scrollView\0"));
        if !scroll.is_null() {
            msg1_ptr(scroll, sel(b"setBackgroundColor:\0"), black);
        }

        // 3. 宿主 UIWindow：安全区那条带子实际是窗口底色
        let ui_window = msg0(wk, sel(b"window\0"));
        if !ui_window.is_null() {
            msg1_ptr(ui_window, sel(b"setBackgroundColor:\0"), black);
        }
    });
}
