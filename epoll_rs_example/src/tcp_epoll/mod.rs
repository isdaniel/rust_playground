use std::{collections::HashMap, io::{self, Read, Write}, net::{TcpListener, TcpStream}, os::fd::{AsRawFd, IntoRawFd, RawFd}};
use crate::syscall;

const READ_FLAGS: i32 = libc::EPOLLONESHOT | libc::EPOLLIN;
const WRITE_FLAGS: i32 = libc::EPOLLONESHOT | libc::EPOLLOUT;
const HTTP_RESP: &[u8] = b"HTTP/1.1 200 OK
content-type: text/html
content-length: 12

Hello World!";

#[derive(Debug)]
pub struct RequestContext {
    pub stream: TcpStream,
    pub content_length: usize,
    pub buf: Vec<u8>,
}

impl RequestContext {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buf: Vec::new(),
            content_length: 0,
        }
    }

    fn read_cb(&mut self, key: u64, epoll_fd: RawFd) -> io::Result<()> {
        let mut buf = [0u8; 4096];
        match self.stream.read(&mut buf) {
            Ok(_) => {
                if let Ok(data) = std::str::from_utf8(&buf) {
                    self.parse_and_set_content_length(data);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
                return Err(e);
            }
        };
        self.buf.extend_from_slice(&buf);
        if self.buf.len() >= self.content_length {
            println!("got all data: {} bytes", self.buf.len());
            modify_interest(epoll_fd, self.stream.as_raw_fd(), listener_write_event(key))?;
        } else {
            modify_interest(epoll_fd, self.stream.as_raw_fd(), listener_read_event(key))?;
        }
        Ok(())
    }

    fn write_cb(&mut self, key: u64, epoll_fd: RawFd) -> io::Result<()> {
        match self.stream.write(HTTP_RESP) {
            Ok(_) => println!("answered from request {}", key),
            Err(e) => eprintln!("could not answer to request {}, {}", key, e),
        };
        self.stream.shutdown(std::net::Shutdown::Both)?;
        let fd = self.stream.as_raw_fd();
        remove_interest(epoll_fd, fd)?;
        Ok(())
    }

    fn parse_and_set_content_length(&mut self, data: &str) {
        if data.contains("HTTP") {
            if let Some(content_length) = data
                .lines()
                .find(|l| l.to_lowercase().starts_with("content-length: "))
            {
                if let Some(len) = content_length
                    .to_lowercase()
                    .strip_prefix("content-length: ")
                {
                    self.content_length = len.parse::<usize>().expect("content-length is valid");
                    println!("set content length: {} bytes", self.content_length);
                }
            }
        }
    }
}

fn close(fd: RawFd) {
    let _ = syscall!(close(fd));
}

pub fn listening_tcp(addr:&str) -> io::Result<(i32,i32)> {
    let listener = TcpListener::bind(addr)?; 
    let mut events : Vec<libc::epoll_event> = Vec::with_capacity(1024);
    let mut request_contexts: HashMap<u64, RequestContext> = HashMap::new();
    let mut key = 100;
    listener.set_nonblocking(true)?;
    let listener_fd = listener.as_raw_fd();
    let epoll_fd = epoll_create()?;
    add_interest(epoll_fd, listener_fd, listener_read_event(key))?;

    loop {
        println!("requests in flight: {}", request_contexts.len());
        events.clear();
        let res = syscall!(epoll_wait(epoll_fd,events.as_mut_ptr() as *mut libc::epoll_event,1024,-1 as libc::c_int))?;
        unsafe { events.set_len(res as usize) };
        for e in &events {
            match e.u64 {
                100 => {
                    match listener.accept() {
                        Ok((stream, addr)) => {
                            stream.set_nonblocking(true)?;
                            println!("new client: {}", addr);
                            key += 1;
                            add_interest(epoll_fd, stream.as_raw_fd(), listener_read_event(key))?;
                            request_contexts.insert(key, RequestContext::new(stream));
                        }
                        Err(e) => eprintln!("couldn't accept: {}", e),
                    };
                    modify_interest(epoll_fd, listener_fd, listener_read_event(100))?;
                }
                key =>{
                    let mut to_delete: Option<u64> = None;
                    if let Some(context) = request_contexts.get_mut(&key) {
                        let events: u32 = e.events;
                        match events {
                            v  if v as i32 & libc::EPOLLIN == libc::EPOLLIN => {
                                context.read_cb(key, epoll_fd)?;
                            }
                            v if v as i32 & libc::EPOLLOUT == libc::EPOLLOUT => {
                                context.write_cb(key, epoll_fd)?;
                                to_delete = Some(key);
                            }
                            v => println!("unexpected events: {}", v),
                        };
                    }
                    if let Some(key) = to_delete {
                        request_contexts.remove(&key);
                    }
                }
            }
        }

    }
}

fn epoll_create() -> io::Result<RawFd>{
    let fd = syscall!(epoll_create1(0))?;
    if let Ok(flag) = syscall!(fcntl(fd, libc::F_GETFD)) {
        syscall!(fcntl(fd, libc::F_SETFD, flag | libc::FD_CLOEXEC));
    }
    Ok(fd)
}


fn listener_read_event(key : u64) -> libc::epoll_event{
    libc::epoll_event{
        events : READ_FLAGS as u32,
        u64: key
    }
}


fn listener_write_event(key: u64) -> libc::epoll_event {
    libc::epoll_event {
        events: WRITE_FLAGS as u32,
        u64: key,
    }
}

fn modify_interest(epoll_fd: RawFd, fd: RawFd, mut event: libc::epoll_event) -> io::Result<()> {
    syscall!(epoll_ctl(epoll_fd, libc::EPOLL_CTL_MOD, fd, &mut event))?;
    Ok(())
}


fn add_interest(epoll_fd: RawFd, fd: RawFd, mut event: libc::epoll_event) -> io::Result<()> {
    syscall!(epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event))?;
    Ok(())
}

fn remove_interest(epoll_fd : RawFd,fd :RawFd) -> io::Result<()> {
    syscall!(epoll_ctl(
        epoll_fd,
        libc::EPOLL_CTL_DEL,
        fd,
        std::ptr::null_mut()
    ))?;
    Ok(())
}