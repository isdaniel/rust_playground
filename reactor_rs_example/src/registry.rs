use std::{collections::{HashMap, HashSet}, io, os::fd::RawFd};
use crate::{syscall, utility::EventId};

const READ_FLAGS: i32 = libc::EPOLLONESHOT | libc::EPOLLIN;
const WRITE_FLAGS: i32 = libc::EPOLLONESHOT | libc::EPOLLOUT;

pub struct Registry {
    epoll_fd: RawFd,
    io_sources: HashMap<RawFd, HashSet<Interest>>,
}

#[derive(PartialEq, Hash, Eq)]
pub enum Interest {
    READ,
    WRITE,
}

impl Registry {
    pub fn new(epoll_fd:RawFd) -> Self {
        Registry {
            epoll_fd,
            io_sources : HashMap::new()
        }
    }

    pub fn register_read(&mut self, fd: RawFd, event_id: EventId) -> io::Result<()> {
        let interests = self.io_sources.entry(fd).or_insert(HashSet::new());

        if interests.is_empty() {
            syscall!(epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut read_event(event_id)
            ))?;
        } else {
            syscall!(epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_MOD,
                fd,
                &mut read_event(event_id)
            ))?;
        }

        interests.clear();
        interests.insert(Interest::READ);

        Ok(())
    }

    pub fn register_write(&mut self, fd: RawFd, event_id: EventId) -> io::Result<()> {
        let interests = self.io_sources.entry(fd).or_insert(HashSet::new());

        if interests.is_empty() {
            syscall!(epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut write_event(event_id)
            ))?;
        } else {
            syscall!(epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_MOD,
                fd,
                &mut write_event(event_id)
            ))?;
        }

        interests.clear();
        interests.insert(Interest::WRITE);

        Ok(())
    }

    pub fn remove_interests(&mut self, fd: RawFd) -> io::Result<()> {
        self.io_sources.remove(&fd);
        syscall!(epoll_ctl(
            self.epoll_fd,
            libc::EPOLL_CTL_DEL,
            fd,
            std::ptr::null_mut()
        ))?;

        Ok(())
    }
}


pub fn read_event(event_id: EventId) -> libc::epoll_event {
    libc::epoll_event {
        events: READ_FLAGS as u32,
        u64: event_id as u64,
    }
}

pub fn write_event(event_id: EventId) -> libc::epoll_event {
    libc::epoll_event {
        events: WRITE_FLAGS as u32,
        u64: event_id as u64,
    }
}