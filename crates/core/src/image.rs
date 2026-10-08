// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};
use ndarray::Array2;

pub const WIDTH: usize = 80;
pub const HEIGHT: usize = 64;
pub const PIXELS: usize = WIDTH * HEIGHT;

pub fn decode(source: &[u8]) -> Result<Vec<u16>> {
    let packed;
    let source = if source.len() == 10560 {
        ensure!(
            source
                .as_chunks::<132>()
                .0
                .iter()
                .all(|c| c[96..].iter().all(|&x| x == 0)),
            "Nonzero image padding"
        );
        packed = source
            .as_chunks::<132>()
            .0
            .iter()
            .flat_map(|c| c[..96].iter().copied())
            .collect::<Vec<_>>();
        &packed[..]
    } else {
        ensure!(source.len() == 7680, "Unexpected image length");
        source
    };
    let mut pixels = vec![0; PIXELS];
    for (i, c) in source.as_chunks::<6>().0.iter().enumerate() {
        let [a, b, c, d, e, f] = *c;
        let values = [
            u16::from(a & 15) * 256 + u16::from(b),
            u16::from(d) * 16 + u16::from(a >> 4),
            u16::from(c) + u16::from(f & 15) * 256,
            u16::from(e) * 16 + u16::from(f >> 4),
        ];
        for (j, value) in values.into_iter().enumerate() {
            let k = i * 4 + j;
            pixels[(k & 63) * WIDTH + (k >> 6)] = value;
        }
    }
    Ok(pixels)
}

pub fn quality(pixels: &[u16], background: &[u16]) -> Result<(f64, f64)> {
    ensure!(
        pixels.len() == PIXELS && background.len() == PIXELS,
        "Expected 80x64 pixels"
    );
    let mut d = Array2::from_shape_vec(
        (HEIGHT, WIDTH),
        background
            .iter()
            .zip(pixels)
            .map(|(&b, &p)| f64::from(b) - f64::from(p))
            .collect(),
    )?;
    let drop = d.sum() / PIXELS as f64;
    for mut row in d.rows_mut() {
        let mut values = row.to_vec();
        let median = *values.select_nth_unstable_by(WIDTH / 2, f64::total_cmp).1;
        row.mapv_inplace(|v| v - median);
    }
    for mut column in d.columns_mut() {
        let mut values = column.to_vec();
        let median = *values.select_nth_unstable_by(HEIGHT / 2, f64::total_cmp).1;
        column.mapv_inplace(|v| v - median);
    }
    let mean = d.sum() / PIXELS as f64;
    let contrast = (d.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / PIXELS as f64).sqrt();
    Ok((drop, contrast))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nibble_layout_and_ec_padding() {
        let mut raw = vec![0; 10560];
        raw[132..138].copy_from_slice(&[0x3a, 0xbc, 0x56, 0x12, 0x78, 0x94]);
        let p = decode(&raw).unwrap();
        assert_eq!([p[1], p[81], p[161], p[241]], [0xabc, 0x123, 0x456, 0x789]);
        raw[96] = 1;
        assert!(decode(&raw).is_err());
    }
    #[test]
    fn invalid_lengths_and_quality() {
        for n in [0, 7679, 8000, 10559] {
            assert!(decode(&vec![0; n]).is_err());
        }
        let p = vec![2000; PIXELS];
        assert_eq!(quality(&p, &p).unwrap(), (0., 0.));
    }
}
