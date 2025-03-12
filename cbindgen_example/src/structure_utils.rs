use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

/// Define a struct that will be shared with C
#[repr(C)]
pub struct Person {
    id: c_int,
    name: *mut c_char, // C-compatible string
}

#[unsafe(no_mangle)]
pub extern "C" fn create_person(id: c_int, name: *const c_char) -> *mut Person {
    if name.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(name) };
    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8 handling
    };

    let person = Box::new(Person {
        id,
        name: CString::new(rust_str).unwrap().into_raw(), // Convert Rust string to C string
    });

    Box::into_raw(person) // Return pointer to heap-allocated `Person`
}

#[unsafe(no_mangle)]
pub extern "C" fn free_person(person:*mut Person){
    if person.is_null(){
        return;
    }

    unsafe {
        let person_box = Box::from_raw(person);
        let _ = CString::from_raw(person_box.name);
    }
}