use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::{WgError, WgResult};

pub struct PacketCodec;

impl Encoder<Bytes> for PacketCodec {
    type Error = WgError;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> WgResult<()> {
        dst.put(item);
        Ok(())
    }
}

impl Decoder for PacketCodec {
    type Item = Bytes;
    type Error = WgError;

    fn decode(&mut self, src: &mut BytesMut) -> WgResult<Option<Self::Item>> {
        if src.len() < 20 {
            return Ok(None);
        }
        let version = src[0] >> 4;
        let len = (match version {
            4 => u16::from_be_bytes(TryInto::<[u8; 2]>::try_into(&src[2..4]).unwrap()),
            6 => u16::from_be_bytes(TryInto::<[u8; 2]>::try_into(&src[4..6]).unwrap()) + 40,
            _ => return Err(WgError::InvalidPacket),
        }) as usize;
        if src.len() < len {
            return Ok(None);
        }
        Ok(Some(src.copy_to_bytes(len)))
    }
}
