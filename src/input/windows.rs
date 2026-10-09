use crossbeam_channel::Sender;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK, RIM_TYPEKEYBOARD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    HWND_MESSAGE, MSG, WM_INPUT, WNDCLASSW,
};

/// Stand-in with the same shape as the Linux handler.
pub struct InputHandler;

// The window procedure has no user data pointer here, so the sender lives in a
// static. Set once in start_reading, read only from the message thread.
static mut PRESS_TX: Option<Sender<()>> = None;

const RI_KEY_BREAK: u16 = 1; // 0 = press, 1 = release (RAWKEYBOARD.Flags)

impl InputHandler {
    /// Windows has no device list to enumerate here: Raw Input delivers
    /// keyboard events from every keyboard to one window. Returns a
    /// placeholder count so main.rs can keep its shape.
    pub fn list_keyboards() -> Vec<()> {
        vec![()]
    }

    pub fn start_reading(_keyboards: Vec<()>, tx: Sender<()>) {
        std::thread::spawn(move || unsafe {
            PRESS_TX = Some(tx);
            run_message_loop();
        });
    }
}

unsafe fn run_message_loop() {
    let class_name = wide("clack_raw_input");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        lpszClassName: class_name.as_ptr(),
        ..std::mem::zeroed()
    };
    RegisterClassW(&wc);

    // Message-only window: receives input, is never shown.
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        0,
        0, 0, 0, 0,
        HWND_MESSAGE,
        null_mut(),
        null_mut(),
        null_mut(),
    );

    let rid = RAWINPUTDEVICE {
        usUsagePage: 0x01, // generic desktop
        usUsage: 0x06,     // keyboard
        dwFlags: RIDEV_INPUTSINK, // receive even when not focused
        hwndTarget: hwnd,
    };
    RegisterRawInputDevices(&rid, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32);

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        let mut size: u32 = 0;
        let hraw = lparam as HRAWINPUT;
        GetRawInputData(hraw, RID_INPUT, null_mut(), &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as u32);

        let mut buf = vec![0u8; size as usize];
        if GetRawInputData(hraw, RID_INPUT, buf.as_mut_ptr() as _, &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as u32) == size
        {
            let raw = &*(buf.as_ptr() as *const RAWINPUT);
            if raw.header.dwType == RIM_TYPEKEYBOARD
                && raw.data.keyboard.Flags & RI_KEY_BREAK == 0
            {
                if let Some(tx) = (*std::ptr::addr_of!(PRESS_TX)).as_ref() {
                    let _ = tx.try_send(());
                }
            }
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}