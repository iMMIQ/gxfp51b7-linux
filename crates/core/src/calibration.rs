// SPDX-License-Identifier: LGPL-3.0-or-later
//! Background-only compatibility reader for existing commissioned deployments.
use anyhow::{Result, ensure};
use ndarray::Array1;
use ndarray_npy::NpzReader;
use std::io::{Read, Seek};

pub fn validate(background: &[u16]) -> Result<()> {
    ensure!(
        background.len() == 5120 && background.iter().all(|&v| v <= 4095),
        "Expected 5120 twelve-bit background pixels"
    );
    Ok(())
}

pub fn legacy_background(reader: impl Read + Seek) -> Result<Vec<u16>> {
    let mut npz = NpzReader::new(reader)?;
    let background: Array1<u16> = npz.by_name("background")?;
    let pixels = background.into_raw_vec_and_offset();
    ensure!(
        pixels.1.is_none_or(|n| n == 0),
        "Contiguous background required"
    );
    validate(&pixels.0)?;
    Ok(pixels.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray_npy::NpzWriter;
    use std::io::Cursor;
    #[test]
    fn migration_reads_background_without_old_matcher_images() {
        for (length, value, valid) in [(5120, 3000, true), (5119, 3000, false), (5120, 4096, false)]
        {
            let mut archive = NpzWriter::new(Cursor::new(Vec::new()));
            archive
                .add_array("background", &Array1::<u16>::from_elem(length, value))
                .unwrap();
            let bytes = archive.finish().unwrap().into_inner();
            assert_eq!(legacy_background(Cursor::new(bytes)).is_ok(), valid);
        }
    }
}
