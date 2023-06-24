use futures_util::stream::SplitStream;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::UnixListener,
};

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
        0
    }

    pub async fn api_set<S: AsyncRead + AsyncWrite>(
        &self,
        reader: &mut SplitStream<Framed<S, LinesCodec>>,
    ) -> i32 {
        while let Some(Ok(line)) = reader.next().await {
            println!("api_set line: {}", line);
        }
        0
    }
}
