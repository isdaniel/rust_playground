use std::os::fd::RawFd;
use libc::*;

#[macro_export]
 macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
    ( $($fn: ident ( $($arg: expr),* $(,)* )),* $(,)* ) => {{
        unsafe{
            $(
                libc::$fn($($arg, )*);
            )*
        }
    }};
}

pub fn set_nonblocking(fd: RawFd) {
    let flags = syscall!(fcntl(fd,F_GETFL)).expect("fcntl fail");
    syscall!(fcntl(fd,F_SETFL, flags | O_NONBLOCK)).expect("fcntl fail");
}


pub fn epoll_create(read_fd: i32) -> i32 {
    // Create epoll instance
    let epfd = syscall!( epoll_create1(0)).expect("epoll_create1 failed");

    // Register read_fd with epoll
    let mut event = epoll_event {
        events: EPOLLIN as u32,
        u64: read_fd as u64,
    };

    syscall!(epoll_ctl(epfd, EPOLL_CTL_ADD, read_fd, &mut event)).expect("epoll_ctl failed");
    epfd
}


pub fn write_message(write_fd: i32,msg : &[u8]) {
    let bytes_written = syscall!(write(write_fd, msg.as_ptr() as *const _, msg.len())).unwrap();
    println!("Wrote {} bytes to pipe", bytes_written);
}
