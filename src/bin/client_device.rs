use std::net::Ipv4Addr;

use wg_rs::device::{allowed_ip::AllowedIP, peer::PeerConfig, Device};

#[tokio::main]
async fn main() {
    let server_addr1 = std::env::var("SERVER_ADDR1").unwrap();
    let server_addr2 = std::env::var("SERVER_ADDR2").unwrap();

    let device = Device::new("utun99".into()).await.unwrap();

    let tcp_stream = tokio::net::TcpStream::connect(server_addr1).await.unwrap();
    let result = device
        .insert_tcp_peer(
            tcp_stream,
            PeerConfig {
                allowed_ips: vec![AllowedIP {
                    addr: Ipv4Addr::from([10, 0, 0, 1]).into(),
                    cidr: 32,
                }],
            },
        )
        .await;
    println!("{result:?}");

    let tcp_stream = tokio::net::TcpStream::connect(server_addr2).await.unwrap();
    let result = device
        .insert_tcp_peer(
            tcp_stream,
            PeerConfig {
                allowed_ips: vec![AllowedIP {
                    addr: Ipv4Addr::from([10, 0, 0, 3]).into(),
                    cidr: 32,
                }],
            },
        )
        .await;
    println!("{result:?}");

    tokio::time::sleep(std::time::Duration::from_secs(500)).await;
}
