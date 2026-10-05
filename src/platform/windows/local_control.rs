use std::{
    collections::VecDeque,
    io, ptr,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
};
use winapi::um::{libloaderapi::GetModuleHandleExA, winuser::*};

static ACTIVE_SESSIONS: AtomicUsize = AtomicUsize::new(0);
static LOCAL_CLAIMED: AtomicBool = AtomicBool::new(false);
static REVISION: AtomicU64 = AtomicU64::new(0);
static HOOK_RUNNING: AtomicBool = AtomicBool::new(false);
const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 4;
const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 2;
const SIGNAL_HISTORY_LIMIT: usize = 32;
const VK_OPEN_BRACKET: u32 = VK_OEM_4 as u32;
const VK_CLOSE_BRACKET: u32 = VK_OEM_6 as u32;

lazy_static::lazy_static! {
    static ref SIGNALS: Mutex<VecDeque<(u64, bool)>> = Default::default();
}

pub(crate) fn session_started() {
    if ACTIVE_SESSIONS.fetch_add(1, Ordering::AcqRel) == 0 {
        LOCAL_CLAIMED.store(false, Ordering::Release);
        REVISION.store(0, Ordering::Release);
        SIGNALS.lock().unwrap().clear();
    }
    if HOOK_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        if let Err(err) = std::thread::Builder::new()
            .name("local-control-hook".to_owned())
            .spawn(run_hook)
        {
            hbb_common::log::warn!("Failed to start local control keyboard hook: {err}");
            HOOK_RUNNING.store(false, Ordering::Release);
        }
    }
}

pub(crate) fn session_ended() {
    if ACTIVE_SESSIONS.fetch_sub(1, Ordering::AcqRel) == 1 {
        LOCAL_CLAIMED.store(false, Ordering::Release);
        REVISION.store(0, Ordering::Release);
        SIGNALS.lock().unwrap().clear();
    }
}

pub(crate) fn signals_since(revision: u64) -> Vec<(u64, bool)> {
    SIGNALS
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|(id, _)| *id > revision)
        .collect()
}

fn run_hook() {
    unsafe {
        let mut module = ptr::null_mut();
        if GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            keyboard_hook as _,
            &mut module,
        ) == 0
        {
            hbb_common::log::warn!(
                "Local control keyboard hook module: {}",
                io::Error::last_os_error()
            );
            HOOK_RUNNING.store(false, Ordering::Release);
            return;
        }
        let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0);
        if hook.is_null() {
            hbb_common::log::warn!(
                "Local control keyboard hook: {}",
                io::Error::last_os_error()
            );
            HOOK_RUNNING.store(false, Ordering::Release);
            return;
        }
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let result = GetMessageA(&mut msg, ptr::null_mut(), 0, 0);
            if result <= 0 {
                if result < 0 {
                    hbb_common::log::warn!(
                        "Local control keyboard hook message loop: {}",
                        io::Error::last_os_error()
                    );
                }
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
        if UnhookWindowsHookEx(hook) == 0 {
            hbb_common::log::warn!(
                "Unhook local control keyboard: {}",
                io::Error::last_os_error()
            );
        }
        HOOK_RUNNING.store(false, Ordering::Release);
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, event: usize, data: isize) -> isize {
    if code >= 0
        && (event as u32 == WM_KEYDOWN || event as u32 == WM_SYSKEYDOWN)
        && ACTIVE_SESSIONS.load(Ordering::Acquire) > 0
    {
        let key = &*(data as *const KBDLLHOOKSTRUCT);
        if key.flags & LLKHF_INJECTED == 0
            && key.dwExtraInfo != enigo::ENIGO_INPUT_EXTRA_VALUE
            && GetAsyncKeyState(VK_CONTROL) < 0
        {
            let claimed = match key.vkCode {
                VK_OPEN_BRACKET => Some(true),
                VK_CLOSE_BRACKET => Some(false),
                _ => None,
            };
            if let Some(claimed) = claimed {
                if LOCAL_CLAIMED.swap(claimed, Ordering::AcqRel) != claimed {
                    let mut signals = SIGNALS.lock().unwrap();
                    let revision = REVISION.fetch_add(1, Ordering::AcqRel) + 1;
                    signals.push_back((revision, claimed));
                    if signals.len() > SIGNAL_HISTORY_LIMIT {
                        signals.pop_front();
                    }
                }
            }
        }
    }
    CallNextHookEx(ptr::null_mut(), code, event, data)
}
