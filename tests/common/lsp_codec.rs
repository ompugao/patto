use serde_json::Value;
use std::io;
use tokio_util::bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Custom LSP Codec for Content-Length framed JSON-RPC
/// Based on ast-grep's implementation
#[derive(Default)]
pub struct LspCodec;

impl Decoder for LspCodec {
    type Item = Value;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src_str = match std::str::from_utf8(&src[..]) {
            Ok(s) => s,
            Err(_) => return Ok(None), // Not valid UTF-8 yet
        };

        let header = "Content-Length: ";
        let header_pos = src_str.find(header);

        if let Some(pos) = header_pos {
            let rest = &src_str[pos + header.len()..];
            let crlf = rest.find("\r\n\r\n");

            if let Some(crlf_pos) = crlf {
                let len_str = &rest[..crlf_pos];
                let content_len: usize = len_str
                    .trim()
                    .parse()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                let body_start = pos + header.len() + crlf_pos + 4;

                if src.len() >= body_start + content_len {
                    let json_bytes = &src[body_start..body_start + content_len];
                    let value = serde_json::from_slice(json_bytes)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    // Remove processed bytes
                    let _ = src.split_to(body_start + content_len);
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }
}

impl Encoder<Value> for LspCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_string(&item)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        dst.put(header.as_bytes());
        dst.put(json.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_decode() {
        let mut codec = LspCodec;
        let mut buf = BytesMut::new();

        let msg = json!({
            "jsonrpc": "2.0",
            "method": "testMethod",
            "params": {
                "key": "value"
            }
        });

        codec.encode(msg.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(decoded, msg);
    }
}
