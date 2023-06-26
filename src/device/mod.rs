use boringtun::noise::{
    errors::WireGuardError, handshake::parse_handshake_anon, rate_limiter::RateLimiter, Packet,
    TunnResult,
};
use dashmap::DashMap;
use rand_core::{OsRng, RngCore};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    sync::{atomic::AtomicU16, Arc},
};

use crate::{
    error::WgResult,
    tun::{codec::PacketCodec, header::IpHeader, stream::TunStream},
    x25519,
};

use self::{
    allowed_ip::AllowedIP,
    peer::{Peer, PeerConfig},
};
use bytes::Bytes;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use ip_network::IpNetwork;
use ip_network_table::IpNetworkTable;
use tokio::{
    net::UdpSocket,
    sync::{Mutex, RwLock},
};
use tokio_util::codec::{Framed, LinesCodec};
pub mod allowed_ip;
pub mod api;
pub mod peer;

pub struct DeviceConfig {
    pub peers: Vec<PeerConfig>,
    pub private_key: [u8; 32],
    pub public_key: [u8; 32],
    pub listen_port: Option<u16>,
}
pub struct Device {
    pub key_pair: RwLock<Option<(x25519::StaticSecret, x25519::PublicKey)>>,
    pub close_sender: tokio::sync::broadcast::Sender<()>,
    pub tun_out: Mutex<SplitSink<Framed<TunStream, PacketCodec>, Bytes>>, // TODO remove lock, use channel
    pub name: String,
    pub next_index: Mutex<IndexLfsr>,
    pub peers: DashMap<x25519::PublicKey, Arc<Mutex<Peer>>>,
    pub peers_by_ip: RwLock<IpNetworkTable<Arc<Mutex<Peer>>>>,
    pub peers_by_idx: DashMap<u32, Arc<Mutex<Peer>>>,
    pub udp4: RwLock<Option<Arc<tokio::net::UdpSocket>>>,
    pub udp6: RwLock<Option<Arc<tokio::net::UdpSocket>>>,
    pub udp_close: tokio::sync::broadcast::Sender<()>,
    pub listen_port: AtomicU16,
    pub rate_limiter: RwLock<Option<Arc<RateLimiter>>>,
}
impl Device {
    pub async fn new(name: String) -> WgResult<Arc<Self>> {
        let tun_stream = TunStream::new(&name)?;
        let mtu = tun_stream.mtu()?;
        let (udp_close, _) = tokio::sync::broadcast::channel(1);
        let (close_sender, mut close_receiver) = tokio::sync::broadcast::channel(1);
        let (tun_out, mut tun_in) = Framed::new(tun_stream, PacketCodec { mtu }).split();
        let this = Arc::new(Self {
            close_sender,
            tun_out: Mutex::new(tun_out),
            name,
            next_index: Default::default(),
            peers: Default::default(),
            peers_by_ip: RwLock::new(IpNetworkTable::new()),
            peers_by_idx: Default::default(),
            udp4: Default::default(),
            udp6: Default::default(),
            key_pair: Default::default(),
            udp_close,
            listen_port: Default::default(),
            rate_limiter: Default::default(),
        });
        this.open_listen_port(8001).await?;
        let (api_listener, api_path) = this.create_api_listener().await?;

        {
            // tunnel input handler
            let device = Arc::clone(&this);
            let mut update_interval = tokio::time::interval(std::time::Duration::from_millis(250));
            let mut rate_limiter_interval =
                tokio::time::interval(std::time::Duration::from_secs(1));

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = update_interval.tick() => {
                            device.update_timers().await;
                        }
                        _ = rate_limiter_interval.tick() => {
                            if let Some(rate_limiter) = device.rate_limiter().await {
                                rate_limiter.reset_count();
                            }
                        }
                        Some(Ok(packet)) = tun_in.next() => {
                            // TODO handle error
                            let _ = device.handle_iface_packet(packet).await;
                        }
                        Ok((api_conn, _)) = api_listener.accept() => {
                            let (mut api_writer, mut api_reader) = Framed::new(api_conn, LinesCodec::new()).split::<String>();
                            if let Some(Ok(line)) = api_reader.next().await {
                                let status = match line.as_str() {
                                    "get=1" => device.api_get(&mut api_writer).await,
                                    "set=1" => device.api_set(&mut api_reader).await,
                                    _ => libc::EIO,
                                };

                                api_writer.send(format!("errno={}\n", status)).await.ok();
                            }
                        }
                        _ = close_receiver.recv() => {
                            device.udp_close.send(()).ok();
                            let _ = tokio::fs::remove_file(&api_path).await;
                            let pub_keys: Vec<_> = device
                                .peers
                                .iter()
                                .map(|entry| entry.key().clone())
                                .collect();
                            for pub_key in pub_keys {
                                device.remove_peer(&pub_key).await;
                            }
                            break;
                        }

                    }
                }
            });
        }
        Ok(this)
    }

    pub async fn update_timers(&self) {
        let udp4 = self.udp4.read().await;
        let udp6 = self.udp6.read().await;
        let (udp4, udp6) = match (udp4.as_ref(), udp6.as_ref()) {
            (Some(udp4), Some(udp6)) => (udp4, udp6),
            _ => return,
        };
        let mut dst_buf = vec![0u8; 65535];
        for peer in self.peers.iter() {
            let mut p = peer.lock().await;
            let endpoint_addr = match p.addr {
                Some(addr) => addr,
                None => continue,
            };
            match p.update_timers(&mut dst_buf[..]) {
                TunnResult::Done => {}
                TunnResult::Err(WireGuardError::ConnectionExpired) => {
                    // p.close(); // close open udp socket
                }
                TunnResult::Err(e) => tracing::error!(message = "Timer error", error = ?e),
                TunnResult::WriteToNetwork(packet) => {
                    match endpoint_addr {
                        SocketAddr::V4(_) => udp4.send_to(packet, &endpoint_addr).await.ok(),
                        SocketAddr::V6(_) => udp6.send_to(packet, &endpoint_addr).await.ok(),
                    };
                }
                _ => panic!("Unexpected result from update_timers"),
            };
        }
    }

    pub async fn handle_incoming_packet(
        &self,
        udp: &UdpSocket,
        addr: SocketAddr,
        packet: &[u8],
        rate_limiter: &RateLimiter,
    ) -> WgResult<()> {
        // self.tun_out.lock().await.send(packet).await

        let mut dst_buf = vec![0u8; 65535];
        let parsed_packet = match rate_limiter.verify_packet(Some(addr.ip()), packet, &mut dst_buf)
        {
            Ok(packet) => packet,
            Err(TunnResult::WriteToNetwork(cookie)) => {
                let _: Result<_, _> = udp.send_to(cookie, &addr).await;
                return Ok(());
            }
            Err(_) => return Ok(()),
        };

        let peer = match &parsed_packet {
            Packet::HandshakeInit(p) => {
                let key_pair = self.key_pair.read().await;
                let (private_key, public_key) = key_pair.as_ref().expect("Key not set");
                parse_handshake_anon(private_key, public_key, p)
                    .ok()
                    .and_then(|hh| {
                        self.peers
                            .get(&x25519::PublicKey::from(hh.peer_static_public))
                            .map(|e| e.value().clone())
                    })
            }
            Packet::HandshakeResponse(p) => self
                .peers_by_idx
                .get(&(p.receiver_idx >> 8))
                .map(|e| e.value().clone()),
            Packet::PacketCookieReply(p) => self
                .peers_by_idx
                .get(&(p.receiver_idx >> 8))
                .map(|e| e.value().clone()),
            Packet::PacketData(p) => self
                .peers_by_idx
                .get(&(p.receiver_idx >> 8))
                .map(|e| e.value().clone()),
        };
        let peer = match peer {
            None => return Ok(()),
            Some(peer) => peer,
        };
        let mut p = peer.lock().await;

        // We found a peer, use it to decapsulate the message+
        let mut flush = false; // Are there packets to send from the queue?
        match p
            .tunnel
            .handle_verified_packet(parsed_packet, &mut dst_buf[..])
        {
            TunnResult::Done => {}
            TunnResult::Err(_) => return Ok(()),
            TunnResult::WriteToNetwork(packet) => {
                flush = true;
                let _: Result<_, _> = udp.send_to(packet, &addr).await;
            }
            TunnResult::WriteToTunnelV4(packet, addr) => {
                // TODO remove clone to_vec
                if p.is_allowed_ip(addr) {
                    let _ = self
                        .tun_out
                        .lock()
                        .await
                        .send(bytes::Bytes::from(packet.to_vec()))
                        .await;

                    // t.iface.write4(packet);
                }
            }
            TunnResult::WriteToTunnelV6(packet, addr) => {
                // TODO remove clone to_vec
                if p.is_allowed_ip(addr) {
                    let _ = self
                        .tun_out
                        .lock()
                        .await
                        .send(bytes::Bytes::from(packet.to_vec()))
                        .await;
                    // t.iface.write6(packet);
                }
            }
        };

        if flush {
            // Flush pending queue
            while let TunnResult::WriteToNetwork(packet) =
                p.tunnel.decapsulate(None, &[], &mut dst_buf[..])
            {
                let _: Result<_, _> = udp.send_to(packet, &addr).await;
            }
        }

        // // This packet was OK, that means we want to create a connected socket for this peer
        // let addr = addr.as_socket().unwrap();
        // let ip_addr = addr.ip();
        // p.set_endpoint(addr);
        // if d.config.use_connected_socket {
        //     if let Ok(sock) = p.connect_endpoint(d.listen_port, d.fwmark) {
        //         d.register_conn_handler(Arc::clone(peer), sock, ip_addr)
        //             .unwrap();
        //     }
        // }

        Ok(())
    }

    pub async fn handle_iface_packet(&self, packet: Bytes) -> WgResult<()> {
        let dst_addr = match IpHeader::from_slice(&packet).map(|h| h.dst_address()) {
            Some(addr) => addr,
            None => return Ok(()), // keepalive
        };
        let peer = match self.peers_by_ip.read().await.longest_match(dst_addr) {
            Some((_, peer)) => peer.clone(),
            None => return Ok(()), // skip
        };
        let mut peer = peer.lock().await;
        // peer.lock().await.send_packet(packet).await?;
        let mut dst_buf = vec![0u8; 65535];
        match peer.tunnel.encapsulate(&packet, &mut dst_buf[..]) {
            TunnResult::Done => {}
            TunnResult::Err(e) => {
                tracing::error!(message = "Encapsulate error", error = ?e)
            }
            TunnResult::WriteToNetwork(packet) => {
                if let Some(addr @ SocketAddr::V4(_)) = peer.addr {
                    let _: Result<_, _> = self
                        .udp4
                        .write()
                        .await
                        .as_ref()
                        .expect("Not connected")
                        .send_to(packet, &addr)
                        .await;
                } else if let Some(addr @ SocketAddr::V6(_)) = peer.addr {
                    let _: Result<_, _> = self
                        .udp6
                        .write()
                        .await
                        .as_ref()
                        .expect("Not connected")
                        .send_to(packet, &addr)
                        .await;
                } else {
                    // tracing::error!("No endpoint");
                }
            }
            _ => panic!("Unexpected result from encapsulate"),
        };

        Ok(())
    }

    pub async fn open_listen_port(self: &Arc<Self>, mut port: u16) -> WgResult<()> {
        let _ = self.udp_close.send(());
        self.udp4.write().await.take();
        self.udp6.write().await.take();

        let udp4 = Arc::new({
            let udp = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            udp.set_reuse_address(true)?;
            udp.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port).into())?;
            udp.set_nonblocking(true)?;
            tokio::net::UdpSocket::from_std(udp.into())?
        });

        if port == 0 {
            port = udp4.local_addr()?.port();
        }
        let udp6 = Arc::new({
            let udp = socket2::Socket::new(
                socket2::Domain::IPV6,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            udp.set_reuse_address(true)?;
            udp.bind(&SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0).into())?;
            udp.set_nonblocking(true)?;
            tokio::net::UdpSocket::from_std(udp.into())?
        });

        {
            let device = self.clone();
            let mut udp_close = self.udp_close.subscribe();
            let udp4 = Arc::clone(&udp4);
            let udp6 = Arc::clone(&udp6);
            let mut udp4_buf = vec![0u8; 65535];
            let mut udp6_buf = vec![0u8; 65535];
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Ok((n, addr)) = udp4.recv_from(&mut udp4_buf[..]) => {
                            let rate_limiter = device.rate_limiter().await.expect("rate limiter not exists");
                            let _ = device.handle_incoming_packet(&udp4,addr,&udp4_buf[..n],&rate_limiter).await;
                        }
                        Ok((n, addr)) = udp6.recv_from(&mut udp6_buf[..]) => {
                            let rate_limiter = device.rate_limiter().await.expect("rate limiter not exists");
                            let _ = device.handle_incoming_packet(&udp6,addr,&udp6_buf[..n],&rate_limiter).await;
                        }
                        _ = udp_close.recv() => {
                            break
                        }
                    }
                }
            });
        }

        self.udp4.write().await.replace(udp4);
        self.udp6.write().await.replace(udp6);
        self.listen_port
            .store(port, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
    // pub async fn insert_tcp_peer(
    //     self: &Arc<Self>,
    //     stream: TcpStream,
    //     config: PeerConfig,
    // ) -> WgResult<()> {
    //     let allowed_ips = config.allowed_ips.clone();
    //     let peer = Peer::new(stream, config, Arc::clone(self)).await?;
    //     let peer = Arc::new(Mutex::new(peer));
    //     for AllowedIP { addr, cidr } in allowed_ips {
    //         self.tcp_router.write().await.insert(
    //             IpNetwork::new_truncate(addr, cidr).expect("cidr is valid length"),
    //             Arc::clone(&peer),
    //         );
    //     }
    //     Ok(())
    // }
    pub fn close(&self) {
        let _ = self.close_sender.send(());
    }

    pub async fn update_peer(&self, config: PeerConfig) {
        if config.remove {
            self.remove_peer(&config.pub_key).await;
        }
        // Update an existing peer
        if self.peers.get(&config.pub_key).is_some() {
            // We already have a peer, we need to merge the existing config into the newly created one
            panic!("Modifying existing peers is not yet supported. Remove and add again instead.");
        }
        let next_index = self.next_index.lock().await.next();
        let device_private = self
            .key_pair
            .read()
            .await
            .as_ref()
            .expect("Private key must be set first")
            .0
            .clone();
        let tunn = boringtun::noise::Tunn::new(
            device_private,
            config.pub_key,
            config.preshared_key,
            config.keepalive,
            next_index,
            None,
        )
        .unwrap();
        let peer = Peer::new(&config, tunn, next_index);

        let peer = Arc::new(Mutex::new(peer));
        self.peers.insert(config.pub_key, Arc::clone(&peer));
        self.peers_by_idx.insert(next_index, Arc::clone(&peer));

        for AllowedIP { addr, cidr } in config.allowed_ips {
            self.peers_by_ip.write().await.insert(
                IpNetwork::new_truncate(addr, cidr).expect("cidr is valid length"),
                Arc::clone(&peer),
            );
        }
    }

    pub async fn remove_peer(&self, pub_key: &x25519::PublicKey) {
        if let Some((_, peer)) = self.peers.remove(pub_key) {
            // Found a peer to remove, now purge all references to it:
            self.peers_by_ip
                .write()
                .await
                .retain(|_, v| !Arc::ptr_eq(&peer, v));

            {
                let p = peer.lock().await;
                self.peers_by_idx.remove(&p.index);
                p.close(); // close open udp socket and free the closure
            }

            // tracing::info!("Peer removed");
        }
    }
    pub async fn set_key(&self, private_key: x25519::StaticSecret) {
        let mut bad_peers = vec![];

        let public_key = x25519::PublicKey::from(&private_key);
        if Some(&public_key) == self.key_pair.read().await.as_ref().map(|p| &p.1) {
            return;
        }
        let rate_limiter = Arc::new(RateLimiter::new(&public_key, 100));

        for peer in self.peers.iter_mut() {
            let peer = peer.value();
            let mut peer_mut = peer.lock().await;

            if peer_mut
                .tunnel
                .set_static_private(
                    private_key.clone(),
                    public_key,
                    Some(Arc::clone(&rate_limiter)),
                )
                .is_err()
            {
                // In case we encounter an error, we will remove that peer
                // An error will be a result of bad public key/secret key combination
                bad_peers.push(Arc::clone(peer));
            }
        }

        self.key_pair
            .write()
            .await
            .replace((private_key.clone(), public_key));
        self.rate_limiter.write().await.replace(rate_limiter);

        // Remove all the bad peers
        for _ in bad_peers {
            unimplemented!();
        }
    }

    async fn clear_peers(&self) {
        self.peers.clear();
        self.peers_by_idx.clear();
        self.peers_by_ip.write().await.retain(|_, _| false);
    }
    async fn rate_limiter(&self) -> Option<Arc<RateLimiter>> {
        self.rate_limiter.read().await.clone()
    }
    fn set_fwmark(&self, fwmark: u32) -> WgResult<()> {
        // TODO
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.close();
    }
}

/// A basic linear-feedback shift register implemented as xorshift, used to
/// distribute peer indexes across the 24-bit address space reserved for peer
/// identification.
/// The purpose is to obscure the total number of peers using the system and to
/// ensure it requires a non-trivial amount of processing power and/or samples
/// to guess other peers' indices. Anything more ambitious than this is wasted
/// with only 24 bits of space.
pub struct IndexLfsr {
    initial: u32,
    lfsr: u32,
    mask: u32,
}

impl IndexLfsr {
    /// Generate a random 24-bit nonzero integer
    pub fn random_index() -> u32 {
        const LFSR_MAX: u32 = 0xffffff; // 24-bit seed
        loop {
            let i = OsRng.next_u32() & LFSR_MAX;
            if i > 0 {
                // LFSR seed must be non-zero
                return i;
            }
        }
    }

    /// Generate the next value in the pseudorandom sequence
    pub fn next(&mut self) -> u32 {
        // 24-bit polynomial for randomness. This is arbitrarily chosen to
        // inject bitflips into the value.
        const LFSR_POLY: u32 = 0xd80000; // 24-bit polynomial
        let value = self.lfsr - 1; // lfsr will never have value of 0
        self.lfsr = (self.lfsr >> 1) ^ ((0u32.wrapping_sub(self.lfsr & 1u32)) & LFSR_POLY);
        assert!(self.lfsr != self.initial, "Too many peers created");
        value ^ self.mask
    }
}

impl Default for IndexLfsr {
    fn default() -> Self {
        let seed = Self::random_index();
        IndexLfsr {
            initial: seed,
            lfsr: seed,
            mask: Self::random_index(),
        }
    }
}
