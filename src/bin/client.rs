use wg_rs::tun::stream::TunStream;
#[tokio::main]
async fn main() {
    let tun_stream = TunStream::new("utun99").unwrap();
    tun_stream.mtu().unwrap();
    let (mut tun_read, mut tun_write) = tokio::io::split(tun_stream);

    let server_addr = std::env::var("SERVER_ADDR").unwrap();
    let tcp_stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    tokio::select! {
        _ = tokio::io::copy(&mut tun_read,&mut tcp_write) => {

        }
        _ = tokio::io::copy(&mut tcp_read,&mut tun_write) => {

        }
    };
}
