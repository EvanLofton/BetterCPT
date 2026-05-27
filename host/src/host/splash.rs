// Native splash — white window with centered progress bar.
// Bridges the ~1s gap between launch and WebView2 rendering.
#![allow(unused_must_use)]

use std::sync::OnceLock;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

const CLASS_NAME: &[u16] = &[
    'B' as u16, 'S' as u16, 'p' as u16, 'l' as u16, 'a' as u16, 's' as u16, 'h' as u16, 0,
];
const WIN_W: i32 = 1000;
const WIN_H: i32 = 680;
const PBAR_W: i32 = 400;
const PBAR_H: i32 = 4;
const PBAR_DURATION_MS: u32 = 500; // native splash lifetime ~0.5s

// Progress: 0.0 → 1.0 over PBAR_DURATION_MS
struct SplashState {
    elapsed_ms: u32,
}

static SPLASH_HWND: OnceLock<isize> = OnceLock::new();

fn centered_pos() -> (i32, i32) {
    unsafe {
        let cx = GetSystemMetrics(SM_CXSCREEN);
        let cy = GetSystemMetrics(SM_CYSCREEN);
        ((cx - WIN_W) / 2, (cy - WIN_H) / 2)
    }
}

unsafe fn paint(hwnd: HWND) {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SplashState;
    if state_ptr.is_null() { return; }
    let state = &*state_ptr;

    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let rect = ps.rcPaint;

    // White background
    FillRect(hdc, &rect, CreateSolidBrush(COLORREF(0x00FFFFFF)));

    // Progress bar — centered
    let px = (WIN_W - PBAR_W) / 2;
    let py = (WIN_H - PBAR_H) / 2;
    let progress = (state.elapsed_ms as f32 / PBAR_DURATION_MS as f32).min(1.0);

    // Track
    FillRect(hdc, &RECT {
        left: px, top: py,
        right: px + PBAR_W, bottom: py + PBAR_H,
    }, CreateSolidBrush(COLORREF(0x00E8E8E8)));

    // Fill
    let fill_w = ((PBAR_W as f32) * progress) as i32;
    if fill_w > 0 {
        FillRect(hdc, &RECT {
            left: px, top: py,
            right: px + fill_w, bottom: py + PBAR_H,
        }, CreateSolidBrush(COLORREF(0x004A6C00)));
    }

    EndPaint(hwnd, &ps);
}

unsafe extern "system" fn splash_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            SetTimer(hwnd, 1, 16, None); // ~60fps
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == 1 => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SplashState;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.elapsed_ms < PBAR_DURATION_MS {
                    state.elapsed_ms += 16;
                }
            }
            InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_PAINT => { paint(hwnd); LRESULT(0) }
        WM_CLOSE => {
            KillTimer(hwnd, 1);
            DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SplashState;
            if !state_ptr.is_null() {
                let _ = Box::from_raw(state_ptr);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn run_splash() {
    unsafe {
        let state = Box::into_raw(Box::new(SplashState { elapsed_ms: 0 }));

        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(splash_wndproc),
            hInstance: hinst.into(),
            lpszClassName: PCWSTR::from_raw(CLASS_NAME.as_ptr()),
            hbrBackground: HBRUSH::default(),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            let _ = Box::from_raw(state);
            return;
        }

        let (x, y) = centered_pos();
        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            PCWSTR::from_raw(CLASS_NAME.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            x, y, WIN_W, WIN_H,
            None, None, hinst,
            Some(state as *const _),
        ) {
            std::result::Result::Ok(h) => h,
            std::result::Result::Err(_) => {
                let _ = Box::from_raw(state);
                return;
            }
        };

        SPLASH_HWND.set(hwnd.0 as isize).ok();
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let got = GetMessageW(&mut msg, None, 0, 0);
            if got.0 == 0 || got.0 == -1 { break; }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn start() {
    std::thread::Builder::new()
        .name("splash".into())
        .spawn(run_splash)
        .ok();
}

pub fn close() {
    if let Some(&hwnd_val) = SPLASH_HWND.get() {
        unsafe {
            PostMessageW(HWND(hwnd_val as *mut _), WM_CLOSE, None, None);
        }
    }
}
