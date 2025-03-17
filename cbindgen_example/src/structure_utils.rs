use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use serde::{Deserialize, Serialize};

/// Define a struct that will be shared with C
#[repr(C)]
pub struct Person {
    id: c_int,
    name: *mut c_char, // C-compatible string
}

#[derive(Serialize, Deserialize)]
pub struct PersonSafe {
    id: c_int,
    name: String, // Owned String instead of raw pointer
}

impl Person{
    pub fn to_safe(&self) -> PersonSafe {
        let name = if self.name.is_null() {
            "".to_string()
        } else {
            let c_str = unsafe{ CStr::from_ptr(self.name) };
            c_str.to_string_lossy().into_owned()
        };

        PersonSafe { id: self.id, name }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn serialize_person(person: *const Person) -> *mut c_char {
    if person.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let safe_person = (*person).to_safe();
        match serde_json::to_string(&safe_person) {
            Ok(json) => CString::new(json).unwrap().into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
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