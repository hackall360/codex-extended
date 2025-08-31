use std::io::{self};

#[derive(Debug, Clone, PartialEq)]
pub enum Resp {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<Resp>>),
}

impl Resp {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Resp::Simple(s) => {
                let mut out = Vec::from(b"+" as &[u8]);
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
                out
            }
            Resp::Error(s) => {
                let mut out = Vec::from(b"-" as &[u8]);
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
                out
            }
            Resp::Integer(i) => {
                let mut out = Vec::from(b":" as &[u8]);
                out.extend_from_slice(i.to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                out
            }
            Resp::Bulk(Some(b)) => {
                let mut out = Vec::from(b"$" as &[u8]);
                out.extend_from_slice(b.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(b);
                out.extend_from_slice(b"\r\n");
                out
            }
            Resp::Bulk(None) => b"$-1\r\n".to_vec(),
            Resp::Array(Some(items)) => {
                let mut out = Vec::from(b"*" as &[u8]);
                out.extend_from_slice(items.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                for it in items {
                    out.extend_from_slice(&it.encode());
                }
                out
            }
            Resp::Array(None) => b"*-1\r\n".to_vec(),
        }
    }

    pub fn parse(input: &[u8]) -> Result<(Resp, usize), io::Error> {
        if input.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""));
        }
        match input[0] {
            b'+' => {
                if let Some(pos) = find_crlf(&input[1..]) {
                    let s = String::from_utf8(input[1..1 + pos].to_vec())
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    Ok((Resp::Simple(s), 1 + pos + 2))
                } else {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                }
            }
            b'-' => {
                if let Some(pos) = find_crlf(&input[1..]) {
                    let s = String::from_utf8(input[1..1 + pos].to_vec())
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    Ok((Resp::Error(s), 1 + pos + 2))
                } else {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                }
            }
            b':' => {
                if let Some(pos) = find_crlf(&input[1..]) {
                    let num = std::str::from_utf8(&input[1..1 + pos])
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                        .parse::<i64>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    Ok((Resp::Integer(num), 1 + pos + 2))
                } else {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                }
            }
            b'$' => {
                if let Some(pos) = find_crlf(&input[1..]) {
                    let len = std::str::from_utf8(&input[1..1 + pos])
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                        .parse::<isize>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let header = 1 + pos + 2;
                    if len == -1 {
                        return Ok((Resp::Bulk(None), header));
                    }
                    let len = len as usize;
                    if input.len() < header + len + 2 {
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""));
                    }
                    let data = input[header..header + len].to_vec();
                    if &input[header + len..header + len + 2] != b"\r\n" {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, ""));
                    }
                    Ok((Resp::Bulk(Some(data)), header + len + 2))
                } else {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                }
            }
            b'*' => {
                if let Some(pos) = find_crlf(&input[1..]) {
                    let len = std::str::from_utf8(&input[1..1 + pos])
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                        .parse::<isize>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let mut offset = 1 + pos + 2;
                    if len == -1 {
                        return Ok((Resp::Array(None), offset));
                    }
                    let len = len as usize;
                    let mut items = Vec::with_capacity(len);
                    for _ in 0..len {
                        let (item, used) = Resp::parse(&input[offset..])?;
                        offset += used;
                        items.push(item);
                    }
                    Ok((Resp::Array(Some(items)), offset))
                } else {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                }
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "")),
        }
    }

    pub fn parse_stream(mut input: &[u8]) -> Result<Vec<Resp>, io::Error> {
        let mut out = Vec::new();
        while !input.is_empty() {
            let (r, used) = Resp::parse(input)?;
            out.push(r);
            input = &input[used..];
        }
        Ok(out)
    }
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}
