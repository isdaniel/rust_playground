use epoll_rs_example::*;
use libc::epoll_event;
use std::io::{self};
use std::{mem};



fn main() -> io::Result<()> {
    // Create a pipe
    let mut fds = [0; 2];

    syscall!(pipe(fds.as_mut_ptr())).expect("pipe failed");

    let read_fd = fds[0];
    let write_fd = fds[1];

    set_nonblocking(read_fd);

    let epfd = epoll_create(read_fd);
    
        // Write some data into pipe to trigger the event
    write_message(write_fd,b"Hello World!");

    // Wait for events
    let mut events: [epoll_event; 10] = unsafe { mem::zeroed() };
    let nfds = syscall!(epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, 5000)).expect("epoll_wait failed");
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
