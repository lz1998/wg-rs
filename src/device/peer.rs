use crate::noise::TunnResult;
use ip_network::IpNetwork;
use ip_network_table::IpNetworkTable;
use std::net::{IpAddr, SocketAddr};

use crate::x25519;

use super::allowed_ip::AllowedIP;

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
    pub pub_key: x25519::PublicKey,
    pub remove: bool,
    pub replace_ips: bool,
    pub endpoint: Option<SocketAddr>,
    pub keepalive: Option<u16>,
    pub preshared_key: Option<[u8; 32]>,
}

impl PeerConfig {
    pub fn new(pub_key: x25519::PublicKey) -> Self {
        PeerConfig {
            pub_key,
            allowed_ips: Vec::new(),
            remove: false,
            replace_ips: false,
            endpoint: None,
            keepalive: None,
            preshared_key: None,
        }
    }
    pub fn remove(&mut self, remove: bool) {
        self.remove = remove
    }
    pub fn replace_ips(&mut self, replace_ips: bool) {
        self.replace_ips = replace_ips
    }
    pub fn endpoint(&mut self, endpoint: SocketAddr) {
        self.endpoint = Some(endpoint)
    }
    pub fn keepalive(&mut self, keepalive: u16) {
        self.keepalive = Some(keepalive)
    }
    pub fn preshared_key(&mut self, preshared_key: [u8; 32]) {
        self.preshared_key = Some(preshared_key)
    }
}

pub struct Peer {
    /// The associated tunnel struct
    pub(crate) tunnel: crate::noise::Tunn,
    /// The index the tunnel uses
    pub index: u32,
    pub addr: Option<SocketAddr>,
    pub allowed_ips: IpNetworkTable<()>,
    pub preshared_key: Option<[u8; 32]>,
}
impl Peer {
    pub fn new(config: &PeerConfig, tunnel: crate::noise::Tunn, index: u32) -> Self {
        let mut allowed_ips = IpNetworkTable::new();
        for AllowedIP { addr, cidr } in config.allowed_ips.iter() {
            allowed_ips.insert(
                IpNetwork::new_truncate(*addr, *cidr).expect("cidr is valid length"),
                (),
            );
        }
        Self {
            tunnel,
            index,
            addr: config.endpoint,
            allowed_ips,
            preshared_key: config.preshared_key,
        }
    }

    pub fn update_timers<'a>(&mut self, dst: &'a mut [u8]) -> TunnResult<'a> {
        self.tunnel.update_timers(dst)
    }

    pub fn is_allowed_ip<I: Into<IpAddr>>(&self, addr: I) -> bool {
        self.allowed_ips.longest_match(addr.into()).is_some()
    }

    // pub async fn send_packet(&mut self, packet: Bytes) -> WgResult<()> {
    //     // TODO encrypt
    //     self.out_stream.send(packet).await
    // }

    pub fn close(&self) {
        // TODO
    }
}
