use std::os::fd::RawFd;
use crate::{registry::Registry, syscall};

pub struct Poll {
    epoll_fd: RawFd,
}

impl Poll {
    pub fn new() -> Self {
        let epoll_fd = syscall!(epoll_create1(0)).expect("failed create epoll");
        
        if let Ok(flag) = syscall!(fcntl(epoll_fd, libc::F_GETFD)) {
            let _ = syscall!(fcntl(epoll_fd, libc::F_SETFD, flag | libc::FD_CLOEXEC));
        }

        Poll {epoll_fd }
    }

    pub fn get_registry(&self) -> Registry {
        Registry::new(self.epoll_fd)
    }

    pub fn poll(&self, events: &mut Vec<libc::epoll_event>) {
        events.clear();
        let res = match syscall!(epoll_wait(
            self.epoll_fd,
            events.as_mut_ptr() as *mut libc::epoll_event,
            1024,
            -1 as libc::c_int,
        )) {
            Ok(v) => v,
            Err(e) => panic!("error during epoll wait: {}", e),
        };

        unsafe { events.set_len(res as usize) };
    }
}

