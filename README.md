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

## Ping test
raw
```bash
PING 10.37.29.241 (10.37.29.241) 56(84) bytes of data.
64 bytes from 10.37.29.241: icmp_seq=1 ttl=64 time=0.703 ms
64 bytes from 10.37.29.241: icmp_seq=2 ttl=64 time=0.703 ms
64 bytes from 10.37.29.241: icmp_seq=3 ttl=64 time=0.693 ms
64 bytes from 10.37.29.241: icmp_seq=4 ttl=64 time=0.683 ms
64 bytes from 10.37.29.241: icmp_seq=5 ttl=64 time=0.641 ms
64 bytes from 10.37.29.241: icmp_seq=6 ttl=64 time=0.657 ms
64 bytes from 10.37.29.241: icmp_seq=7 ttl=64 time=0.667 ms
64 bytes from 10.37.29.241: icmp_seq=8 ttl=64 time=0.699 ms
64 bytes from 10.37.29.241: icmp_seq=9 ttl=64 time=0.675 ms
64 bytes from 10.37.29.241: icmp_seq=10 ttl=64 time=0.718 ms

--- 10.37.29.241 ping statistics ---
10 packets transmitted, 10 received, 0% packet loss, time 1833ms
rtt min/avg/max/mdev = 0.641/0.683/0.718/0.022 ms
```

boringtun
```bash
PING 10.0.0.1 (10.0.0.1) 56(84) bytes of data.
64 bytes from 10.0.0.1: icmp_seq=1 ttl=64 time=13.9 ms
64 bytes from 10.0.0.1: icmp_seq=2 ttl=64 time=5.64 ms
64 bytes from 10.0.0.1: icmp_seq=3 ttl=64 time=5.85 ms
64 bytes from 10.0.0.1: icmp_seq=4 ttl=64 time=5.70 ms
64 bytes from 10.0.0.1: icmp_seq=5 ttl=64 time=5.88 ms
64 bytes from 10.0.0.1: icmp_seq=6 ttl=64 time=5.68 ms
64 bytes from 10.0.0.1: icmp_seq=7 ttl=64 time=5.70 ms
64 bytes from 10.0.0.1: icmp_seq=8 ttl=64 time=6.16 ms
64 bytes from 10.0.0.1: icmp_seq=9 ttl=64 time=5.78 ms
64 bytes from 10.0.0.1: icmp_seq=10 ttl=64 time=5.94 ms

--- 10.0.0.1 ping statistics ---
10 packets transmitted, 10 received, 0% packet loss, time 1808ms
rtt min/avg/max/mdev = 5.637/6.627/13.945/2.443 ms
```

wg-rs
```bash
PING 10.0.0.1 (10.0.0.1) 56(84) bytes of data.
64 bytes from 10.0.0.1: icmp_seq=1 ttl=64 time=1.19 ms
64 bytes from 10.0.0.1: icmp_seq=2 ttl=64 time=1.15 ms
64 bytes from 10.0.0.1: icmp_seq=3 ttl=64 time=1.13 ms
64 bytes from 10.0.0.1: icmp_seq=4 ttl=64 time=1.14 ms
64 bytes from 10.0.0.1: icmp_seq=5 ttl=64 time=1.20 ms
64 bytes from 10.0.0.1: icmp_seq=6 ttl=64 time=1.23 ms
64 bytes from 10.0.0.1: icmp_seq=7 ttl=64 time=1.16 ms
64 bytes from 10.0.0.1: icmp_seq=8 ttl=64 time=1.14 ms
64 bytes from 10.0.0.1: icmp_seq=9 ttl=64 time=1.11 ms
64 bytes from 10.0.0.1: icmp_seq=10 ttl=64 time=1.19 ms

--- 10.0.0.1 ping statistics ---
10 packets transmitted, 10 received, 0% packet loss, time 1804ms
rtt min/avg/max/mdev = 1.113/1.163/1.230/0.034 ms
```

## Speed test
```bash
dd if=/dev/zero of=a bs=$(echo "300*1024*1024" | bc) count=1 &> /dev/null
scp ./a 10.0.0.1:~/a
```

- raw 254.0MB/s
- boringtun 407.0KB/s
- wg-rs 47.1MB/s
