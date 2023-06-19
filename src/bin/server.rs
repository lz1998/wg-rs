use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::Framed;
use wg_rs::tun::{codec::PacketCodec, header::IpHeader, stream::TunStream};
#[tokio::main]
async fn main() {
    let mut tun_stream = TunStream::new("utun99").unwrap();
    tun_stream.mtu().unwrap();

    // sudo ip addr add 10.0.0.2/24 dev utun99 && sudo ip link set utun99 up
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let ping_packet = vec![
        69u8, 0, 0, 84, 252, 217, 64, 0, 64, 1, 41, 205, 10, 0, 0, 1, 10, 0, 0, 2, 8, 0, 150, 3,
        219, 128, 0, 3, 68, 238, 141, 100, 0, 0, 0, 0, 244, 82, 1, 0, 0, 0, 0, 0, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42,
        43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
    ];

    let (_, mut tun_write) = tokio::io::split(tun_stream);
    let result = tun_write.write(&ping_packet).await;
    println!("1");
    dbg!(result);

    // let (mut tun_out, mut tun_in) = Framed::new(tun_stream, PacketCodec).split();
    // let result = tun_out.send(bytes::Bytes::from(ping_packet)).await;
    // println!("2");
    // dbg!(result);
}
