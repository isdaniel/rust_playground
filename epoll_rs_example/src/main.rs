use epoll_rs_example::{example_epoll::*, tcp_epoll::listening_tcp};
use std::{io::{self}, net::TcpListener};
use std::os::unix::io::{AsRawFd, RawFd};
use std::io::prelude::*;

fn main() -> io::Result<()> {

    // let fd_context = epoll_create();
    // // Write some data into pipe to trigger the event
    // write_message(fd_context.write_fd,b"Hello World!");

    // // Wait for events
    // epoll_wait(&fd_context);
    // close_fd(&fd_context);

    listening_tcp("127.0.0.1:8080")?;
    Ok(())
}
