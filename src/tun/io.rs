use std::io::{Read, Write};
use std::os::raw::c_char;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

#[derive(Debug)]
pub struct TunIo(RawFd);

impl FromRawFd for TunIo {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(fd)
    }
}

impl AsRawFd for TunIo {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl TunIo {
    pub fn open() -> std::io::Result<Self> {
        static TUN: &[u8] = b"/dev/net/tun\0";
        let fd = match unsafe {
            libc::open(
                TUN.as_ptr().cast::<c_char>(),
                libc::O_RDWR | libc::O_NONBLOCK,
            )
        } {
            -1 => return Err(std::io::Error::last_os_error()),
            fd => fd,
        };
        Ok(Self(fd))
    }

    pub fn close(&self) {
        unsafe { libc::close(self.0) };
    }
}

impl Drop for TunIo {
    fn drop(&mut self) {
        self.close()
    }
}

impl Read for TunIo {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = unsafe { libc::read(self.0, buf.as_ptr() as *mut _, buf.len() as _) };
        if n < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(n as _)
    }
}

impl Write for TunIo {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = unsafe { libc::write(self.0, buf.as_ptr() as *const _, buf.len() as _) };
        if n < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(n as _)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_io() {
        let tun = TunIo::open().expect("failed to open tun");
        let async_fd = tokio::io::unix::AsyncFd::new(tun).expect("failed to get async_fd");

        dbg!(async_fd);
    }
}
