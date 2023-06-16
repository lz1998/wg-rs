use futures_util::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use wg_rs::tun::{codec::PacketCodec, header::IpHeader, stream::TunStream};
#[tokio::main]
async fn main() {
    let tun_stream = TunStream::new("utun99").unwrap();
    tun_stream.mtu().unwrap();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    let (tcp_stream, addr) = listener.accept().await.unwrap();
    println!("{addr:?}");

    let (mut tun_out, mut tun_in) = Framed::new(tun_stream, PacketCodec).split();
    let (mut tcp_out, mut tcp_in) = Framed::new(tcp_stream, PacketCodec).split();

    loop {
        tokio::select! {
            Some(Ok(buf)) = tun_in.next() => {
                let header=IpHeader::from_slice(&buf).unwrap();
                println!("TUN_IN: {:?} => {:?}, len: {}", header.src_address(),header.dst_address(),header.computed_len());
                tcp_out.send(buf).await.unwrap();
            }
            Some(Ok(buf)) = tcp_in.next() => {
                let header=IpHeader::from_slice(&buf).unwrap();
                println!("TCP_IN: {:?} => {:?}, len: {}", header.src_address(),header.dst_address(),header.computed_len());
                tun_out.send(buf).await.unwrap();
            }
        }
    }
}
