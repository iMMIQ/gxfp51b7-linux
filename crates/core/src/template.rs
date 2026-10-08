// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};
use ndarray::{Array1, Array3, Axis};
use ndarray_npy::{NpzReader, NpzWriter};
use std::io::{Read, Seek, Write};

pub struct Template {
    pub background: Array1<u16>,
    pub images: Array3<f64>,
}
impl Template {
    pub fn read<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut npz = NpzReader::new(reader)?;
        let result = Self {
            background: npz.by_name("background")?,
            images: npz.by_name("images")?,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.background.shape() == [5120] && self.images.shape() == [15, 56, 72],
            "Unexpected template shape"
        );
        ensure!(
            self.background.iter().all(|&v| v <= 4095) && self.images.iter().all(|v| v.is_finite()),
            "Invalid template values"
        );
        Ok(())
    }
    pub fn write<W: Write + Seek>(&self, writer: W) -> Result<W> {
        self.validate()?;
        let mut npz = NpzWriter::new(writer);
        npz.add_array("background", &self.background)?;
        npz.add_array("images", &self.images)?;
        Ok(npz.finish()?)
    }
    pub fn references(&self) -> Vec<ndarray::Array2<f64>> {
        self.images
            .axis_iter(Axis(0))
            .map(|a| a.to_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn round_trip_and_invalid_values() {
        let mut t = Template {
            background: Array1::from_elem(5120, 3000),
            images: Array3::from_shape_fn((15, 56, 72), |(r, y, x)| ((r + y + x) as f64).sin()),
        };
        let bytes = t.write(Cursor::new(Vec::new())).unwrap();
        let restored = Template::read(Cursor::new(bytes.into_inner())).unwrap();
        assert_eq!(t.background, restored.background);
        assert_eq!(t.images, restored.images);
        t.images[[0, 0, 0]] = f64::NAN;
        assert!(t.write(Cursor::new(Vec::new())).is_err());
        t.images[[0, 0, 0]] = 0.;
        t.background[0] = 4096;
        assert!(t.validate().is_err());
    }
}
