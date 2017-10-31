use iovec::IoVec;
use iovec::unix as iovec;
use libc;
use std::{cmp, io, mem, ptr};
use std::os::unix::io::RawFd;

fn cvt(r: libc::ssize_t) -> io::Result<usize> {
    if r == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(r as usize)
    }
}

// Convert return of -1 into error message, handling retry on EINTR
fn cvt_r<F: FnMut() -> libc::ssize_t>(mut f: F) -> io::Result<usize> {
    loop {
        match cvt(f()) {
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
            other => return other
        }
    }
}

pub fn recv_msg_with_flags(
    socket: RawFd,
    bufs: &mut [&mut IoVec],
    cmsg: &mut [u8],
    flags: libc::c_int
) -> io::Result<(usize, usize, libc::c_int)> {
    let slice = iovec::as_os_slice_mut(bufs);
    let len = cmp::min(<libc::c_int>::max_value() as usize, slice.len());
    let (control, controllen) = if cmsg.is_empty() {
        (ptr::null_mut(), 0)
    } else {
        (cmsg.as_ptr() as *mut _, cmsg.len())
    };

    let mut msghdr: libc::msghdr = unsafe { mem::zeroed() };
    msghdr.msg_name = ptr::null_mut();
    msghdr.msg_namelen = 0;
    msghdr.msg_iov = slice.as_mut_ptr();
    msghdr.msg_iovlen = len as _;
    msghdr.msg_control = control;
    msghdr.msg_controllen = controllen as _;

    let n = try!(cvt_r(|| unsafe {
        libc::recvmsg(socket, &mut msghdr as *mut _, flags)
    }));

    let controllen = msghdr.msg_controllen as usize;
    Ok((n, controllen, msghdr.msg_flags))
}

pub fn send_msg_with_flags(
    socket: RawFd,
    bufs: &[&IoVec],
    cmsg: &[u8],
    flags: libc::c_int
) -> io::Result<usize> {
    let slice = iovec::as_os_slice(bufs);
    let len = cmp::min(<libc::c_int>::max_value() as usize, slice.len());
    let (control, controllen) = if cmsg.is_empty() {
        (ptr::null_mut(), 0)
    } else {
        (cmsg.as_ptr() as *mut _, cmsg.len())
    };

    let mut msghdr: libc::msghdr = unsafe { mem::zeroed() };
    msghdr.msg_name = ptr::null_mut();
    msghdr.msg_namelen = 0;
    msghdr.msg_iov = slice.as_ptr() as *mut _;
    msghdr.msg_iovlen = len as _;
    msghdr.msg_control = control;
    msghdr.msg_controllen = controllen as _;

    cvt_r(|| unsafe {
        libc::sendmsg(socket, &msghdr as *const _, flags)
    })
}
