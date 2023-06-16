use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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
#[repr(C)]
#[derive(Debug)]
pub struct Ipv4Header {
    pub v_ih: u8,
    pub ts: u8,
    pub tl: u16,
    pub id: u16,
    pub fl_fo: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub cusm: u16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}
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

#[repr(C)]
#[derive(Debug)]
pub struct Ipv6Header {
    pub v_tc_fl: u32,
    pub len: u16,
    pub nh: u8,
    pub hl: u8,
    pub src: [u8; 16],
    pub dst: [u8; 16],
}

#[derive(Debug)]
pub enum IpHeader<'a> {
    V4(&'a Ipv4Header),
    V6(&'a Ipv6Header),
}

impl<'a> IpHeader<'a> {
    pub fn from_slice(data: &'a [u8]) -> Option<IpHeader<'a>> {
        if data.is_empty() {
            // keepalive
            return None;
        }
        let version = data[0] >> 4;
        if (version == 4 && data.len() < 20) || (version == 6 && data.len() < 40) {
            return None;
        }
        unsafe {
            match version {
                // TODO: check align
                4 => Some(Self::V4(
                    std::mem::transmute::<&'a [u8; 20], &'a Ipv4Header>(
                        (&data[..20]).try_into().unwrap(), //TODO: handle err
                    ),
                )),
                6 => Some(Self::V6(
                    std::mem::transmute::<&'a [u8; 40], &'a Ipv6Header>(
                        (&data[..40]).try_into().unwrap(), //TODO: handle err
                    ),
                )),
                _ => None,
            }
        }
    }

    pub fn src_address(&self) -> IpAddr {
        match self {
            Self::V4(header) => Ipv4Addr::from(header.src).into(),
            Self::V6(header) => Ipv6Addr::from(header.src).into(),
        }
    }

    pub fn dst_address(&self) -> IpAddr {
        match self {
            Self::V4(header) => Ipv4Addr::from(header.dst).into(),
            Self::V6(header) => Ipv6Addr::from(header.dst).into(),
        }
    }

    pub fn computed_len(&self) -> usize {
        (match self {
            Self::V4(header) => header.tl,
            Self::V6(header) => header.len,
        }) as usize
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
