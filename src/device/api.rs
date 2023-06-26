use futures_util::stream::SplitStream;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::UnixListener,
};

use crate::{key_bytes::KeyBytes, x25519};

use super::*;

const SOCK_DIR: &str = "/var/run/wireguard/";

impl Device {
    pub async fn create_api_listener(&self) -> WgResult<(UnixListener, String)> {
        let _ = tokio::fs::create_dir_all(SOCK_DIR).await;
        let path = format!("{}/{}.sock", SOCK_DIR, self.name);
        let _ = tokio::fs::remove_file(&path).await;
        let api_listener = tokio::net::UnixListener::bind(&path)?;
        Ok((api_listener, path))
    }

    pub async fn api_get<S: AsyncRead + AsyncWrite>(
        &self,
        _writer: &mut SplitSink<Framed<S, LinesCodec>, String>,
    ) -> i32 {
        // TODO
        println!("api_get");
        0
    }

    pub async fn api_set<S: AsyncRead + AsyncWrite>(
        self: &Arc<Self>,
        reader: &mut SplitStream<Framed<S, LinesCodec>>,
    ) -> i32 {
        while let Some(Ok(cmd)) = reader.next().await {
            if cmd.is_empty() {
                return 0; // Done
            }
            let parsed_cmd: Vec<&str> = cmd.split('=').collect();
            if parsed_cmd.len() != 2 {
                return libc::EPROTO;
            }

            let (key, val) = (parsed_cmd[0], parsed_cmd[1]);
            match key {
                "private_key" => match val.parse::<KeyBytes>() {
                    Ok(key_bytes) => self.set_key(x25519::StaticSecret::from(key_bytes.0)).await,
                    Err(_) => return libc::EINVAL,
                },
                "listen_port" => match val.parse::<u16>() {
                    Ok(port) => match self.open_listen_port(port).await {
                        Ok(()) => {}
                        Err(_) => return libc::EADDRINUSE,
                    },
                    Err(_) => return libc::EINVAL,
                },
                #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
                "fwmark" => match val.parse::<u32>() {
                    Ok(mark) => match self.set_fwmark(mark) {
                        Ok(()) => {}
                        Err(_) => return libc::EADDRINUSE,
                    },
                    Err(_) => return libc::EINVAL,
                },
                "replace_peers" => match val.parse::<bool>() {
                    Ok(true) => self.clear_peers().await,
                    Ok(false) => {}
                    Err(_) => return libc::EINVAL,
                },
                "public_key" => match val.parse::<KeyBytes>() {
                    // Indicates a new peer section
                    Ok(key_bytes) => {
                        return self
                            .api_set_peer(reader, x25519::PublicKey::from(key_bytes.0))
                            .await
                    }
                    Err(_) => return libc::EINVAL,
                },
                _ => return libc::EINVAL,
            }
        }
        0
    }

    pub async fn api_set_peer<S: AsyncRead + AsyncWrite>(
        &self,
        reader: &mut SplitStream<Framed<S, LinesCodec>>,
        pub_key: x25519::PublicKey,
    ) -> i32 {
        let mut config = PeerConfig::new(pub_key);
        while let Some(Ok(cmd)) = reader.next().await {
            if cmd.is_empty() {
                self.update_peer(config).await;
                return 0; // Done
            }
            let parsed_cmd: Vec<&str> = cmd.splitn(2, '=').collect();
            if parsed_cmd.len() != 2 {
                return libc::EPROTO;
            }
            let (key, val) = (parsed_cmd[0], parsed_cmd[1]);
            match key {
                "remove" => match val.parse::<bool>() {
                    Ok(true) => config.remove = true,
                    Ok(false) => config.remove = false,
                    Err(_) => return libc::EINVAL,
                },
                "preshared_key" => match val.parse::<KeyBytes>() {
                    Ok(key_bytes) => config.preshared_key = Some(key_bytes.0),
                    Err(_) => return libc::EINVAL,
                },
                "endpoint" => match val.parse::<SocketAddr>() {
                    Ok(addr) => config.endpoint = Some(addr),
                    Err(_) => return libc::EINVAL,
                },
                "persistent_keepalive_interval" => match val.parse::<u16>() {
                    Ok(interval) => config.keepalive = Some(interval),
                    Err(_) => return libc::EINVAL,
                },
                "replace_allowed_ips" => match val.parse::<bool>() {
                    Ok(true) => config.replace_ips = true,
                    Ok(false) => config.replace_ips = false,
                    Err(_) => return libc::EINVAL,
                },
                "allowed_ip" => match val.parse::<AllowedIP>() {
                    Ok(ip) => config.allowed_ips.push(ip),
                    Err(_) => return libc::EINVAL,
                },
                "public_key" => {
                    // Indicates a new peer section. Commit changes for current peer, and continue to next peer
                    self.update_peer(config).await;
                    match val.parse::<KeyBytes>() {
                        Ok(key_bytes) => config = PeerConfig::new(key_bytes.0.into()),
                        Err(_) => return libc::EINVAL,
                    }
                }
                "protocol_version" => match val.parse::<u32>() {
                    Ok(1) => {} // Only version 1 is legal
                    _ => return libc::EINVAL,
                },
                _ => return libc::EINVAL,
            }
        }
        0
    }
}
