extern crate winapi;

use std::ptr;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{EnumWindows, FindWindowW, FindWindowExW, SendMessageTimeoutA};

pub fn get_worker_window_handle() -> Result<HWND, ()> {
    unsafe {
        create_new_workerW_window();

        let mut workerw: HWND = ptr::null_mut();

        // Enumerate all top-level windows
        EnumWindows(Some(enum_windows_proc), &mut workerw as *mut HWND as isize);

        if workerw.is_null() {
            Err(())
        } else {
            Ok(workerw)
        }
    }
}

pub fn send_cleanup_message() {
    unsafe {
        SendMessageTimeoutA(
            get_worker_window_handle().unwrap(),
            0xC107,
            0,
            0,
            0,
            1000,
            ptr::null_mut(),
        );
    }
}

// EnumWindows callback function
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: isize) -> i32 {
    let mut workerw: HWND = ptr::null_mut();
    
    // Find SHELLDLL_DefView child window
    let class_name_def_view = to_wstring("SHELLDLL_DefView");
    let p = FindWindowExW(hwnd, ptr::null_mut(), class_name_def_view.as_ptr(), ptr::null());

    if !p.is_null() {
        println!("Found DefView: {:?}", p);
        // Find next sibling WorkerW window
        let class_name_workerw = to_wstring("WorkerW");
        workerw = FindWindowExW(ptr::null_mut(), hwnd, class_name_workerw.as_ptr(), ptr::null());
        println!("Found WorkerW: {:?}", workerw);
        *(lparam as *mut HWND) = workerw;
    }

    // Store the WorkerW handle through the pointer passed via lparam

    // Return true to continue enumeration
    1
}

// This should create a new window with the same parent as the shell's default view
fn create_new_workerW_window() {
    unsafe {
        // Fetch the Progman window handle (Progman)
        let progman = find_window("Progman");

        if progman.is_null() {
            println!("Failed to find Progman window.");
            return;
        }

        let mut result: usize = 0;

        // Send WM_WININICHANGE (0x052C) to Progman
        let res = SendMessageTimeoutA(
            progman,
            0x052C,
            0 as usize,
            0,
            winapi::um::winuser::SMTO_NORMAL,
            1000,
            &mut result as *mut usize,
        );

        if res == 0 {
            println!("SendMessageTimeout failed.");
        } else {
            println!("SendMessageTimeout succeeded. Result: {:?}", result);
        }
    }
}

// Helper function to find window by class name
unsafe fn find_window(class_name: &str) -> HWND {
    let class_name = to_wstring(class_name);
    let window_name = ptr::null();

    FindWindowW(class_name.as_ptr(), window_name)
}

// Helper function to convert Rust strings to wide strings
fn to_wstring(str: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(str).encode_wide().chain(std::iter::once(0)).collect()
}
