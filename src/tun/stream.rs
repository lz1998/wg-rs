use super::io::TunIo;
use futures::ready;
use libc::*;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[repr(C)]
union IfrIfru {
    ifru_addr: sockaddr,
    ifru_addr_v4: sockaddr_in,
    ifru_addr_v6: sockaddr_in,
    ifru_dstaddr: sockaddr,
    ifru_broadaddr: sockaddr,
    ifru_flags: c_short,
    ifru_metric: c_int,
    ifru_mtu: c_int,
    ifru_phys: c_int,
    ifru_media: c_int,
    ifru_intval: c_int,
    //ifru_data: caddr_t,
    //ifru_devmtu: ifdevmtu,
    //ifru_kpi: ifkpi,
    ifru_wake_flags: u32,
    ifru_route_refcnt: u32,
    ifru_cap: [c_int; 2],
    ifru_functional_type: u32,
}

#[repr(C)]
pub struct ifreq {
    ifr_name: [c_uchar; IFNAMSIZ],
    ifr_ifru: IfrIfru,
}
#[derive(Debug)]
pub struct TunStream {
    pub name: String,
    pub fd: AsyncFd<TunIo>,
}
impl TunStream {
    pub fn new(name: &str) -> std::io::Result<Self> {
        let io = TunIo::open()?;

        let mut req = ifreq {
            ifr_name: [0; IFNAMSIZ],
            ifr_ifru: IfrIfru {
                ifru_flags: (IFF_TUN | IFF_NO_PI | IFF_MULTI_QUEUE) as _,
            },
        };
        req.ifr_name[..name.as_bytes().len()].copy_from_slice(name.as_bytes());
        if unsafe { ioctl(io.as_raw_fd(), 0x4004_54ca as _, &req) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // unsafe { tunsetiff(tun_io.as_raw_fd(), &req as *const _ as _) }?;
        Ok(TunStream {
            fd: AsyncFd::new(io)?,
            name: name.to_string(),
        })
    }
    pub fn mtu(&self) -> std::io::Result<usize> {
        let fd = match unsafe { socket(AF_INET, SOCK_STREAM, IPPROTO_IP) } {
            -1 => return Err(std::io::Error::last_os_error()),
            fd => fd,
        };

        let mut ifr = ifreq {
            ifr_name: [0; IF_NAMESIZE],
            ifr_ifru: IfrIfru { ifru_mtu: 0 },
        };

        ifr.ifr_name[..self.name.as_bytes().len()].copy_from_slice(self.name.as_bytes());

        if unsafe { ioctl(fd, SIOCGIFMTU as _, &ifr) } < 0 {
            return Err(std::io::Error::last_os_error());
        }

        unsafe { close(fd) };

        Ok(unsafe { ifr.ifr_ifru.ifru_mtu } as _)
    }
}

impl AsyncRead for TunStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.fd.poll_read_ready_mut(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_mut().read(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for TunStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.fd.poll_write_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.fd.poll_write_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().flush()) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;
    #[tokio::test]
    async fn test_tun() {
        let tun = TunStream::new("utun106").unwrap();
        let mut tun = dbg!(tun);
        let mtu = tun.mtu();
        let _ = dbg!(mtu);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut buf = vec![0u8; 65536];
        loop {
            match tun.read(&mut buf).await {
                Ok(n) => {
                    println!("{:x?}", &buf[0..n]);
                }
                Err(e) => {
                    println!("{e}")
                }
            }
        }
    }

    #[tokio::test]
    async fn test_boringtun() {
        let tun = boringtun::device::tun::TunSocket::new("utun123").unwrap();
        let tun = dbg!(tun);
        let mtu = tun.mtu().unwrap();
        dbg!(mtu);
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
