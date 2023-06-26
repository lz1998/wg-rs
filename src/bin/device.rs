use wg_rs::device::Device;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _d = Device::new(String::from("utun99")).await.unwrap();

    std::future::pending::<()>().await;
}
