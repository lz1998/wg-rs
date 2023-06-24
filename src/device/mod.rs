use std::sync::Arc;

use crate::{
    error::WgResult,
    tun::{codec::PacketCodec, header::IpHeader, stream::TunStream},
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
    net::TcpStream,
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
    pub tcp_router: RwLock<IpNetworkTable<Arc<Mutex<Peer<TcpStream>>>>>,
    pub close_sender: tokio::sync::broadcast::Sender<()>,
    pub tun_out: Mutex<SplitSink<Framed<TunStream, PacketCodec>, Bytes>>, // TODO remove lock, use channel
    pub cleanup_paths: Mutex<Vec<String>>,
    pub name: String,
}
impl Device {
    pub async fn new(name: String) -> WgResult<Arc<Self>> {
        let tun_stream = TunStream::new(&name)?;
        tun_stream.mtu()?;
        let (close_sender, mut close_receiver) = tokio::sync::broadcast::channel(1);
        let (tun_out, mut tun_in) = Framed::new(tun_stream, PacketCodec).split();

        let this = Arc::new(Self {
            tcp_router: RwLock::new(IpNetworkTable::new()),
            close_sender,
            tun_out: Mutex::new(tun_out),
            cleanup_paths: Default::default(),
            name,
        });

        let (api_listener, api_path) = this.create_api_listener().await?;

        {
            // tunnel input handler
            let device = Arc::clone(&this);
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(Ok(packet)) = tun_in.next() => {
                            // TODO handle error
                            let _ = device.handle_iface_packet(packet).await;
                        }
                        Ok((api_conn,_)) = api_listener.accept() => {
                            let (mut api_writer, mut api_reader) = Framed::new(api_conn, LinesCodec::new()).split::<String>();
                            if let Some(Ok(line)) = api_reader.next().await {
                                let status = match line.as_str() {
                                    "get=1" => device.api_get(&mut api_writer).await,
                                    "set=1" => device.api_set(&mut api_reader).await,
                                    _ => libc::EIO,
                                };
                                api_writer.send(format!("errno={}", status)).await.ok();
                            }
                        }
                        _ = close_receiver.recv() => {
                            let _ = tokio::fs::remove_file(&api_path).await;
                            for (_,peer) in device.tcp_router.write().await.iter(){
                                peer.lock().await.close()
                            }
                            break;
                        }

                    }
                }
            });
        }
        Ok(this)
    }

    pub async fn handle_incoming_packet(&self, packet: Bytes) -> WgResult<()> {
        self.tun_out.lock().await.send(packet).await
    }

    pub async fn handle_iface_packet(&self, packet: Bytes) -> WgResult<()> {
        let dst_addr = match IpHeader::from_slice(&packet).map(|h| h.dst_address()) {
            Some(addr) => addr,
            None => return Ok(()), // keepalive
        };
        let peer = match self.tcp_router.read().await.longest_match(dst_addr) {
            Some((_, peer)) => peer.clone(),
            None => return Ok(()), // skip
        };
        peer.lock().await.send_packet(packet).await?;
        Ok(())
    }

    pub async fn insert_tcp_peer(
        self: &Arc<Self>,
        stream: TcpStream,
        config: PeerConfig,
    ) -> WgResult<()> {
        let allowed_ips = config.allowed_ips.clone();
        let peer = Peer::new(stream, config, Arc::clone(self)).await?;
        let peer = Arc::new(Mutex::new(peer));
        for AllowedIP { addr, cidr } in allowed_ips {
            self.tcp_router.write().await.insert(
                IpNetwork::new_truncate(addr, cidr).expect("cidr is valid length"),
                Arc::clone(&peer),
            );
        }
        Ok(())
    }
    pub fn close(&self) {
        let _ = self.close_sender.send(());
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.close();
    }
}
