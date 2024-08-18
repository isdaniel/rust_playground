extern "C" {
    fn c_hello(name: *const i8);
    fn c_add(a: i32, b: i32) -> i32;
}

fn main() {
    let name = std::ffi::CString::new("Daniel").unwrap();
    unsafe {
        c_hello(name.as_ptr());
    }

    let sum = unsafe { c_add(5, 7) };
    println!("Sum from C: {}", sum);
}