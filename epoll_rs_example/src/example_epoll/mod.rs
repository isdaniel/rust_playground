use std::{mem, os::fd::RawFd};
use libc::*;
use crate::syscall;
pub struct EpollContext {
    pub read_fd : i32,
    pub write_fd : i32,
    pub epfd : i32,
}

impl EpollContext{
    fn new(read_fd: i32, write_fd:i32, epfd: i32) -> EpollContext{
        EpollContext {
            read_fd,
            write_fd,
            epfd
        }
    }
}

pub fn set_nonblocking(fd: RawFd) {
    let flags = syscall!(fcntl(fd,F_GETFL)).expect("fcntl fail");
    syscall!(fcntl(fd,F_SETFL, flags | O_NONBLOCK)).expect("fcntl fail");
}


pub fn epoll_create() -> EpollContext {
    let mut fds = [0; 2];
    syscall!(pipe(fds.as_mut_ptr())).expect("pipe failed");
    let read_fd = fds[0];
    let write_fd = fds[1];
    set_nonblocking(read_fd);
    let epfd =  syscall!( epoll_create1(0)).expect("epoll_create1 failed");
    // Register read_fd with epoll
    let mut event = epoll_event {
        events: EPOLLIN as u32,
        u64: read_fd as u64,
    };

    syscall!(epoll_ctl(epfd, EPOLL_CTL_ADD, read_fd, &mut event)).expect("epoll_ctl failed");
    EpollContext::new(read_fd, write_fd, epfd)
}

pub fn write_message(write_fd: i32,msg : &[u8]) {
    let bytes_written = syscall!(write(write_fd, msg.as_ptr() as *const _, msg.len())).unwrap();
    println!("Wrote {} bytes to pipe", bytes_written);
}



pub fn close_fd(fd_context: &EpollContext) {
    syscall!(close(fd_context.read_fd),close(fd_context.write_fd),close(fd_context.epfd));
}
    
pub fn epoll_wait(fd_context: &EpollContext) {
    let mut events: [epoll_event; 10] = unsafe { mem::zeroed() };
    let nfds = syscall!(epoll_wait(fd_context.epfd, events.as_mut_ptr(), events.len() as i32, 5000)).expect("epoll_wait failed");
    println!("epoll_wait reported {} events", nfds);

    for i in 0..nfds as usize {
        let ev = events[i];
        if ev.u64 == fd_context.read_fd as u64 {
            let mut buf = [0u8; 64];
            let n = syscall!(read(fd_context.read_fd, buf.as_mut_ptr() as *mut _, buf.len())).unwrap();
            if n > 0 {
                let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
                println!("Received: {}", s);
            }
        }
    }
}
