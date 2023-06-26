# wg-rs

tokio wireguard/boringtun
## Usage

### Build and Run
```bash
cargo build --release
sudo ./target/release/device
```
### Generate key pair for each endpoint
```bash
# Generate key pair in ./privatekey and ./publickey
umask 077
wg genkey > privatekey # sudo apt install -y wireguard
wg pubkey < privatekey > publickey
```

### Endpoint A
myconfig.conf
```conf
[Interface]
PrivateKey = <PRIVATE_KEY_A>
ListenPort = <PORT_A>

[Peer]
PublicKey = <PUBLIC_KEY_B>
Endpoint = <IP_B>:<PORT_B>
AllowedIPs = 10.0.0.1/32
```

```bash
sudo wg setconf utun99 myconfig.conf && sudo ip addr add 10.0.0.2/24 dev utun99 && sudo ip link set utun99 up
```

### Endpoint B
myconfig.conf
```conf
[Interface]
PrivateKey = <PRIVATE_KEY_B>
ListenPort = <PORT_B>

[Peer]
PublicKey = <PUBLIC_KEY_A>
Endpoint = <IP_A>:<PORT_A>
AllowedIPs = 10.0.0.2/32
```

```bash
sudo wg setconf utun99 myconfig.conf && sudo ip addr add 10.0.0.1/24 dev utun99 && sudo ip link set utun99 up
```
