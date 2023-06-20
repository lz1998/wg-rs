use bytes::Bytes;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use ip_network::IpNetwork;
use ip_network_table::IpNetworkTable;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::error::WgResult;
use crate::tun::codec::PacketCodec;
use crate::tun::header::IpHeader;

use super::allowed_ip::AllowedIP;
use super::Device;

#[derive(Clone, Debug)]
pub struct PeerConfig {
    // pub endpoint: std::net::SocketAddr,
    pub allowed_ips: Vec<AllowedIP>,
    // pub remove: bool,
    // pub public_key: [u8; 32],
    // preshared_key
    // persistent_keepalive_interval
    // replace_allowed_ips
    // protocol_version
}
pub struct Peer<S: AsyncRead + AsyncWrite + Send + 'static> {
    pub out_stream: SplitSink<Framed<S, PacketCodec>, Bytes>,
    pub close_sender: tokio::sync::broadcast::Sender<()>,
}
impl<S: AsyncRead + AsyncWrite + Send + 'static> Peer<S> {
    pub async fn new(stream: S, config: PeerConfig, device: Arc<Device>) -> WgResult<Self> {
        let mut ip_filter = IpNetworkTable::new();
        for AllowedIP { addr, cidr } in config.allowed_ips {
            ip_filter.insert(
                IpNetwork::new_truncate(addr, cidr).expect("cidr is valid length"),
                (),
            );
        }
        // let tcp_stream = TcpStream::connect(config.endpoint).await?;
        let (out_stream, mut in_stream) = Framed::new(stream, PacketCodec).split();
        let (close_sender, mut close_receiver) = tokio::sync::broadcast::channel(1);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(Ok(packet)) = in_stream.next() => {
                        let src_address=match IpHeader::from_slice(&packet) {
                            Some(header) => header.src_address(),
                            None => continue
                        };
                        if ip_filter.longest_match(src_address).is_some() {
                            // TODO handle error
                            let _ = device.handle_incoming_packet(packet).await;
                        }
                    }
                    // TODO register tcp_in handler
                    _ = close_receiver.recv() => {
                        break;
                    }
                }
            }
        });
        Ok(Self {
            out_stream,
            close_sender,
        })
    }

    pub async fn send_packet(&mut self, packet: bytes::Bytes) -> WgResult<()> {
        // TODO encrypt
        self.out_stream.send(packet).await
    }

    pub fn close(&self) {
        let _ = self.close_sender.send(());
    }
}

impl Peer<TcpStream> {}

impl<S: AsyncRead + AsyncWrite + Send + 'static> Drop for Peer<S> {
    fn drop(&mut self) {
        self.close()
    }
}
