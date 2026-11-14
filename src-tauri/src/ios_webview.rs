//! iOS：WKWebView 自动安全区内缩与底色的修正。
//!
//! UIKit 默认 `contentInsetAdjustmentBehavior = automatic`，会按安全区再给页面套一层
//! content inset：视口被缩小「顶安全区 + 底安全区」，而页面仍从屏幕左上角开始画，
//! 于是底部露出 WebView / 窗口的底色（浅色模式下就是一条白边）。
//! 项目 CSS 里已经用 `--sat` / `--sab` 处理过一次安全区（见 App.tsx 的 MobileShell），
//! 原生再缩一次等于重复留白，所以这里把自动内缩关掉（`.never`），
//! 页面铺满全屏，`env(safe-area-inset-*)` 继续由视图本身提供。
//!
//! 另外把 WKWebView / UIScrollView 的底色铺成画布色，避免任何缝隙（启动瞬间、
//! 视频页上下留白）露出系统白底。

use std::ffi::{c_char, c_void};

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

pub fn disable_auto_insets(app: &tauri::App) {
    use tauri::Manager;

    let Some(webview) = app.get_webview("main") else {
        return;
    };

    let _ = webview.with_webview(|pw| unsafe {
        let wk = pw.inner();
        if wk.is_null() {
            return;
        }

        let msg0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg1_usize: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
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

        // 1. 关掉 UIKit 的自动内缩：
        //    UIScrollViewContentInsetAdjustmentBehavior.automatic = 0
        //    .scrollableAxes = 1, .never = 2, .always = 3
        let scroll = msg0(wk, sel(b"scrollView\0"));
        if !scroll.is_null() {
            msg1_usize(scroll, sel(b"setContentInsetAdjustmentBehavior:\0"), 2);
        }

        // 2. 底色铺成画布色 #1a1a1a（0x1a / 255）
        let color_cls = objc_getClass(b"UIColor\0".as_ptr() as *const c_char);
        if !color_cls.is_null() {
            let bg = msg4(
                color_cls,
                sel(b"colorWithRed:green:blue:alpha:\0"),
                26.0 / 255.0,
                26.0 / 255.0,
                26.0 / 255.0,
                1.0,
            );
            if !bg.is_null() {
                msg1_ptr(wk, sel(b"setBackgroundColor:\0"), bg);
                if !scroll.is_null() {
                    msg1_ptr(scroll, sel(b"setBackgroundColor:\0"), bg);
                }
            }
        }

        // 3. WebView 不再不透明，露出上面设的底色（而不是系统的白）
        msg1_bool(wk, sel(b"setOpaque:\0"), 0);
    });
}
