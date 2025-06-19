use epoll_rs_example::example_epoll::*;
use std::io::{self};

fn main() -> io::Result<()> {

    let fd_context = epoll_create();
    // Write some data into pipe to trigger the event
    write_message(fd_context.write_fd,b"Hello World!");

    // Wait for events
    epoll_wait(&fd_context);
    close_fd(&fd_context);

    Ok(())
}
