use libc::*;
use std::io::{self};
use std::os::unix::io::{RawFd};
use std::{mem};

#[allow(unused_macros)]
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

fn set_nonblocking(fd: RawFd) {
    let flags = syscall!(fcntl(fd,F_GETFL)).expect("fcntl fail");
    syscall!(fcntl(fd,F_SETFL, flags | O_NONBLOCK)).expect("fcntl fail");
}

fn main() -> io::Result<()> {
    // Create a pipe
    let mut fds = [0; 2];

    syscall!(pipe(fds.as_mut_ptr())).expect("pipe failed");

    let read_fd = fds[0];
    let write_fd = fds[1];

    set_nonblocking(read_fd);

    // Create epoll instance
    let epfd = syscall!( epoll_create1(0)).expect("epoll_create1 failed");

    // Register read_fd with epoll
    let mut event = epoll_event {
        events: EPOLLIN as u32,
        u64: read_fd as u64,
    };

    syscall!(epoll_ctl(epfd, EPOLL_CTL_ADD, read_fd, &mut event)).expect("epoll_ctl failed");

    // Write some data into pipe to trigger the event
    let msg = b"hello epoll";
    let bytes_written = syscall!(write(write_fd, msg.as_ptr() as *const _, msg.len())).unwrap();
    println!("Wrote {} bytes to pipe", bytes_written);

    // Wait for events
    let mut events: [epoll_event; 10] = unsafe { mem::zeroed() };
    let nfds = syscall!(epoll_wait(epfd, events.as_mut_ptr(), 10, 5000)).expect("epoll_wait failed");
    println!("epoll_wait reported {} events", nfds);

    for i in 0..nfds as usize {
        let ev = events[i];
        if ev.u64 == read_fd as u64 {
            let mut buf = [0u8; 64];
            let n = syscall!(read(read_fd, buf.as_mut_ptr() as *mut _, buf.len())).unwrap();
            if n > 0 {
                let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
                println!("Received: {}", s);
            }
        }
    }

    syscall!(close(read_fd),close(write_fd),close(epfd));

    Ok(())
}
