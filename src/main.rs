use std::{mem::*, sync::mpsc::sync_channel, thread};
use winapi::{
    ctypes::*,
    shared::minwindef::*,
    um::{processthreadsapi::GetCurrentThreadId, winuser::*},
};

const MOVE_LEFT: u16 = 0x41; // A key
const MOVE_RIGHT: u16 = 0x44; // D key
const MOVE_UP: u16 = 0x57; // W key
const MOVE_DOWN: u16 = 0x53; // S key
const JUMP: u16 = VK_UP as u16; // Up arrow
const SHOOT: u16 = VK_RIGHT as u16; // Right arrow
const RESTART: u16 = VK_LEFT as u16; // Left arrow
const JUMP_CANCEL: u16 = VK_DOWN as u16; // Down arrow

fn main() {
    let keyboard_thread = unsafe { generate_hook() };

    while !stop_key_pressed() {}

    unsafe { PostThreadMessageA(keyboard_thread, WM_QUIT, 0, 0) };
}

unsafe fn generate_hook() -> u32 {
    let (keyboard_sender, keyboard_receiver) = sync_channel(0);

    thread::spawn(move || {
        let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard), std::ptr::null_mut(), 0);

        let mut msg = zeroed();

        keyboard_sender.send(GetCurrentThreadId()).unwrap();

        while GetMessageA(&mut msg, zeroed(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }

        UnhookWindowsHookEx(hook);
    });

    keyboard_receiver.recv().unwrap()
}

unsafe fn send_key(key: u16, pressed: bool) {
    let dw_flags = if pressed { 0 } else { KEYEVENTF_KEYUP };

    let mut keybd_input: INPUT_u = zeroed();

    *keybd_input.ki_mut() = KEYBDINPUT {
        wVk: key,
        dwExtraInfo: 0,
        wScan: 0,
        time: 0,
        dwFlags: dw_flags,
    };

    let mut input = INPUT {
        type_: INPUT_KEYBOARD,
        u: keybd_input,
    };

    SendInput(1, &mut input, size_of::<INPUT>() as i32);
}

unsafe extern "system" fn keyboard(n_code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let info = *transmute::<LPARAM, PKBDLLHOOKSTRUCT>(l_param);
    if info.flags & LLKHF_INJECTED == 0 {
        let converted_key = match info.vkCode as u16 {
            MOVE_LEFT => VK_LEFT,
            MOVE_RIGHT => VK_RIGHT,
            MOVE_UP => VK_UP,
            MOVE_DOWN => VK_DOWN,
            JUMP => VK_SHIFT,
            SHOOT => 0x5A,
            RESTART => 0x52,
            JUMP_CANCEL => VK_SHIFT,
            _ => return CallNextHookEx(zeroed(), n_code, w_param, l_param),
        } as u16;

        let pressed = if info.vkCode as u16 == JUMP_CANCEL {
            false
        } else {
            match w_param as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => true,
                WM_KEYUP | WM_SYSKEYUP => false,
                _ => std::hint::unreachable_unchecked(),
            }
        };

        send_key(converted_key, pressed);

        return -1;
    }

    CallNextHookEx(zeroed(), n_code, w_param, l_param)
}

pub fn stop_key_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_CONTROL) < 0 && GetAsyncKeyState(0x51) < 0 }
}
