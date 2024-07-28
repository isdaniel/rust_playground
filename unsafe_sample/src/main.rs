fn main() {
    let mut num = 5;

    let r1 = &num as *const i32;
    let r2 = &mut num as *mut i32;

    // thetrait bound i32 pointer is not satisfied
    // println!("r1: {:p}", r1);
    // println!("r1: {:p}", *r1);

    unsafe {
        println!("r1: {}", *r1);
        println!("r2: {}", *r2);
    }

    // let address = 0x012345usize;
    // let r = address as *const i32;
    //might be a bad address
    // unsafe {
    //     println!("r: {}", *r);
    // }

    unsafe {
        dangerous();
    }

    let mut v = vec![1,2,3,4,5,6];
    let (first,second) = split_at_mut(&mut v, 3);
    println!("{:?}", first);
    println!("{:?}", second);

    println!("=====================");

    unsafe {
        println!("abs(-3): {}", abs(-3));
    }
}

fn split_at_mut(slice: &mut [i32], mid: usize) -> (&mut [i32], &mut [i32]) {
    let len = slice.len();
    let ptr = slice.as_mut_ptr();
    assert!(mid <= len);
    unsafe {
        (
            std::slice::from_raw_parts_mut(ptr, mid),
            std::slice::from_raw_parts_mut(ptr.add(mid), len - mid),
        )
    }
}

//unsafe functions
unsafe fn dangerous() {
    println!("dangerous function");
}

//ABI, Application Binary Interface, defines how to call a function at the assembly level
//Rust uses the C ABI by default
extern "C" {
    fn abs(input: i32) -> i32;
}

// export a function to C code  https://doc.rust-lang.org/nomicon/ffi.html
#[no_mangle]
pub  extern "C" fn call_from_c() {
    println!("Just called a Rust function from C!");
}