use std::net::IpAddr;

// if len == 0:
//     keepalive  TODO
// else:

// ipv4
//
//  0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |Version|  IHL  |Type of Service|          Total Length         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |         Identification        |Flags|      Fragment Offset    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |  Time to Live |    Protocol   |         Header Checksum       |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                       Source Address                          |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                    Destination Address                        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                    Options                    |    Padding    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

// ipv6
//                      1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |Version| Traffic Class |           Flow Label                  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |         Payload Length        |  Next Header  |   Hop Limit   |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                                                               |
// +                                                               +
// |                                                               |
// +                         Source Address                        +
// |                                                               |
// +                                                               +
// |                                                               |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                                                               |
// +                                                               +
// |                                                               |
// +                      Destination Address                      +
// |                                                               |
// +                                                               +
// |                                                               |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug)]
pub struct IpHeader<'a> {
    pub version: u8,
    pub data: &'a [u8],
}

impl<'a> IpHeader<'a> {
    pub fn from_slice(data: &'a [u8]) -> Option<Self> {
        if data.is_empty() {
            return None; // keepalive
        }
        let version = data[0] >> 4;
        if !((version == 4 && data.len() >= 20) || (version == 6 && data.len() >= 40)) {
            return None;
        }
        Some(Self { version, data })
    }

    pub fn src_address(&self) -> IpAddr {
        match self.version {
            4 => IpAddr::from(TryInto::<[u8; 4]>::try_into(&self.data[12..16]).unwrap()),
            6 => IpAddr::from(TryInto::<[u8; 16]>::try_into(&self.data[8..24]).unwrap()),
            _ => unreachable!(),
        }
    }
    pub fn dst_address(&self) -> IpAddr {
        match self.version {
            4 => IpAddr::from(TryInto::<[u8; 4]>::try_into(&self.data[16..20]).unwrap()),
            6 => IpAddr::from(TryInto::<[u8; 16]>::try_into(&self.data[24..40]).unwrap()),
            _ => unreachable!(),
        }
    }

    pub fn computed_len(&self) -> u16 {
        match self.version {
            4 => u16::from_be_bytes(TryInto::<[u8; 2]>::try_into(&self.data[2..4]).unwrap()),
            6 => u16::from_be_bytes(TryInto::<[u8; 2]>::try_into(&self.data[4..6]).unwrap()) + 40,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_from_slice_v4() {
        let mut data = vec![0; 40];
        data[0] = 0x40;
        let v = data[0] >> 4;
        println!("{v}");
        let header = IpHeader::from_slice(&data);
        dbg!(header);
    }

    #[test]
    fn test_from_slice_v6() {
        let mut data = vec![0; 40];
        data[0] = 0x60;
        let v = data[0] >> 4;
        println!("{v}");
        let header = IpHeader::from_slice(&data);
        dbg!(header);
    }
}
