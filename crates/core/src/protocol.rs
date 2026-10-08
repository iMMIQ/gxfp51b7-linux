// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};

pub fn encode(command: u8, payload: &[u8]) -> Result<Vec<u8>> {
    let len = u16::try_from(payload.len() + 1)?;
    let mut body = vec![command];
    body.extend(len.to_le_bytes());
    body.extend(payload);
    let sum = body.iter().fold(0u8, |s, &v| s.wrapping_add(v));
    body.push(0xaa_u8.wrapping_sub(sum));
    let mut packet = vec![0xa0];
    packet.extend(u16::try_from(body.len())?.to_le_bytes());
    let sum = packet.iter().fold(0u8, |s, &v| s.wrapping_add(v));
    packet.push(sum);
    packet.extend(body);
    Ok(packet)
}

pub fn decode(raw: &[u8]) -> Option<(u8, &[u8])> {
    if raw.len() < 8 || raw[0] != 0xa0 {
        return None;
    }
    let n = usize::from(u16::from_le_bytes([raw[1], raw[2]])) + 4;
    if n < 8 || n > raw.len() || raw[..3].iter().fold(0u8, |s, &v| s.wrapping_add(v)) != raw[3] {
        return None;
    }
    let raw = &raw[..n];
    if usize::from(u16::from_le_bytes([raw[5], raw[6]])) + 7 != n
        || raw[4..].iter().fold(0u8, |s, &v| s.wrapping_add(v)) != 0xaa
    {
        return None;
    }
    Some((raw[4], &raw[7..n - 1]))
}

pub fn tls_record(raw: &[u8]) -> Option<&[u8]> {
    if raw.len() < 4
        || raw[0] != 0xb0
        || raw[..3].iter().fold(0u8, |s, &v| s.wrapping_add(v)) != raw[3]
    {
        return None;
    }
    let n = usize::from(u16::from_le_bytes([raw[1], raw[2]]));
    if n < 5 || n > raw.len() - 4 {
        return None;
    }
    Some(&raw[4..4 + n])
}

pub fn rpc_header(raw: &[u8; 12]) -> Result<(u32, u32)> {
    let magic = u32::from_le_bytes(raw[..4].try_into()?);
    ensure!(magic == 0x43505247, "Invalid guest RPC magic");
    Ok((
        u32::from_le_bytes(raw[4..8].try_into()?),
        u32::from_le_bytes(raw[8..].try_into()?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_and_corruption() {
        assert_eq!(
            encode(0x90, &[]).unwrap(),
            [0xa0, 4, 0, 0xa4, 0x90, 1, 0, 0x19]
        );
        let raw = [0xa0, 6, 0, 0xa6, 0x90, 3, 0, 0xe0, 0xe1, 0x56];
        assert_eq!(decode(&raw), Some((0x90, &[0xe0, 0xe1][..])));
        for i in 0..raw.len() {
            let mut r = raw;
            r[i] ^= 1;
            assert_eq!(decode(&r), None);
            assert_eq!(decode(&raw[..i]), None);
        }
    }
    #[test]
    fn lengths_and_rpc_are_bounded() {
        assert!(encode(1, &vec![0; 65535]).is_err());
        assert!(rpc_header(&[0; 12]).is_err());
        assert!(tls_record(&[0xb0, 255, 255, 0xae]).is_none());
    }
}
