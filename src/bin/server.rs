use wg_rs::tun::stream::TunStream;
#[tokio::main]
async fn main() {
    let tun_stream = TunStream::new("utun99").unwrap();
    tun_stream.mtu().unwrap();
    let (mut tun_read, mut tun_write) = tokio::io::split(tun_stream);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    let (tcp_stream, addr) = listener.accept().await.unwrap();
    println!("{addr:?}");
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    tokio::select! {
        _ = tokio::io::copy(&mut tun_read, &mut tcp_write) => {

        }
        _ = tokio::io::copy(&mut tcp_read, &mut tun_write) => {

        }
    };
}
