use futures_util::StreamExt;
use tokio_util::codec::Framed;
use wg_rs::tun::{codec::PacketCodec, stream::TunStream};

#[tokio::main]
async fn main() {
    let tun_stream = TunStream::new("utun99").unwrap();
    tun_stream.mtu().unwrap();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    let (tcp_stream, addr) = listener.accept().await.unwrap();
    println!("{addr:?}");

    let (tun_out, tun_in) = Framed::new(tun_stream, PacketCodec).split();
    let (tcp_out, tcp_in) = Framed::new(tcp_stream, PacketCodec).split();

    tokio::select! {
        end = tcp_in.forward(tun_out) => {
            println!("{end:?}");
        }
        end = tun_in.forward(tcp_out) => {
            println!("{end:?}");
        }
    }
}
