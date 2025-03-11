use std::ffi::{c_char, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn string_to_uint32(str: *const c_char, number: *mut u32) -> bool {
    if str.is_null() || number.is_null() {
        return false;
    }

    let c_str = unsafe { CStr::from_ptr(str) };

    if let Some(num) = c_str.to_str().ok().and_then(|s| s.parse::<u32>().ok()) {
        unsafe { *number = num };
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
