use wg_rs::device::Device;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _d = Device::new(String::from("utun99")).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(500)).await;
}
