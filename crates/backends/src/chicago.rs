// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{CStr, c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
const PIXELS: usize = 5120;
const CAPACITY: usize = 2 * 1024 * 1024;
#[repr(C)]
#[derive(Default, Debug, Clone, Serialize)]
pub struct Evidence {
    pub reject: i32,
    pub score: i32,
    pub features: u32,
    pub quality: u32,
    pub coverage: u32,
    pub count: u32,
    pub accepted: u32,
    pub position_x: u32,
    pub position_y: u32,
    pub position_reject: u32,
}
#[derive(Serialize, Deserialize)]
pub struct Print {
    pub version: u32,
    pub background: Vec<u16>,
    pub calibration: Vec<u8>,
    pub gallery: Vec<u8>,
}
unsafe extern "C" {
    fn gxfp_chicago_new(
        background: *const u16,
        calibration: *const u8,
        calibration_len: usize,
        gallery: *const u8,
        gallery_len: usize,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn gxfp_chicago_free(context: *mut c_void);
    fn gxfp_chicago_process(
        context: *mut c_void,
        raw: *const u16,
        enroll: i32,
        output: *mut Evidence,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn gxfp_chicago_export(
        context: *mut c_void,
        calibration: i32,
        output: *mut u8,
        capacity: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> usize;
}
/// Exclusive ownership of native state. Each instance stays on its creating thread.
pub struct Chicago {
    context: NonNull<c_void>,
    background: Vec<u16>,
    _local: PhantomData<Rc<()>>,
}
fn valid(raw: &[u16]) -> Result<()> {
    ensure!(
        raw.len() == PIXELS && raw.iter().all(|&v| v <= 4095),
        "Expected 5120 twelve-bit pixels"
    );
    Ok(())
}
fn error(buffer: &[c_char; 512]) -> anyhow::Error {
    // SAFETY: the bridge writes a bounded NUL-terminated message; the buffer starts zeroed.
    anyhow::anyhow!(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    )
}
impl Chicago {
    pub fn new(background: &[u16]) -> Result<Self> {
        Self::create(background, &[], &[])
    }
    pub fn from_print(print: &Print) -> Result<Self> {
        ensure!(
            print.version == 1 && !print.calibration.is_empty() && !print.gallery.is_empty(),
            "Unsupported Chicago print"
        );
        Self::create(&print.background, &print.calibration, &print.gallery)
    }
    fn create(background: &[u16], calibration: &[u8], gallery: &[u8]) -> Result<Self> {
        valid(background)?;
        ensure!(
            calibration.len() <= CAPACITY && gallery.len() <= CAPACITY,
            "Oversized Chicago print"
        );
        let mut message = [0; 512];
        // SAFETY: all slice pointers remain valid for the call. Native code copies inputs
        // and returns an exclusively owned context, freed once by Drop.
        let ptr = unsafe {
            gxfp_chicago_new(
                background.as_ptr(),
                calibration.as_ptr(),
                calibration.len(),
                gallery.as_ptr(),
                gallery.len(),
                message.as_mut_ptr(),
                message.len(),
            )
        };
        Ok(Self {
            context: NonNull::new(ptr).ok_or_else(|| error(&message))?,
            background: background.to_vec(),
            _local: PhantomData,
        })
    }
    pub fn enroll(&mut self, raw: &[u16]) -> Result<Evidence> {
        self.process(raw, true)
    }
    pub fn verify(&mut self, raw: &[u16]) -> Result<Evidence> {
        self.process(raw, false)
    }
    fn process(&mut self, raw: &[u16], enroll: bool) -> Result<Evidence> {
        valid(raw)?;
        let mut output = Evidence::default();
        let mut message = [0; 512];
        // SAFETY: &mut self ensures exclusive context access. The input has validated
        // dimensions and output/message buffers match the bridge ABI and bounds.
        let result = unsafe {
            gxfp_chicago_process(
                self.context.as_ptr(),
                raw.as_ptr(),
                i32::from(enroll),
                &mut output,
                message.as_mut_ptr(),
                message.len(),
            )
        };
        ensure!(result != 0, "{}", error(&message));
        Ok(output)
    }
    fn export(&mut self, calibration: bool) -> Result<Vec<u8>> {
        let mut bytes = vec![0; CAPACITY];
        let mut message = [0; 512];
        // SAFETY: exclusive context and valid writable bounded byte/message buffers.
        let length = unsafe {
            gxfp_chicago_export(
                self.context.as_ptr(),
                i32::from(calibration),
                bytes.as_mut_ptr(),
                bytes.len(),
                message.as_mut_ptr(),
                message.len(),
            )
        };
        ensure!(length > 0 && length <= bytes.len(), "{}", error(&message));
        bytes.truncate(length);
        Ok(bytes)
    }
    pub fn print(&mut self) -> Result<Print> {
        Ok(Print {
            version: 1,
            background: self.background.clone(),
            calibration: self.export(true)?,
            gallery: self.export(false)?,
        })
    }
}
impl Drop for Chicago {
    fn drop(&mut self) {
        // SAFETY: this is the sole owner and drops the context exactly once.
        unsafe { gxfp_chicago_free(self.context.as_ptr()) };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_sensor_and_invalid_input() {
        let mut c = Chicago::new(&vec![3000; PIXELS]).unwrap();
        assert_ne!(c.enroll(&vec![3000; PIXELS]).unwrap().reject, 0);
        assert!(c.verify(&[0; 64]).is_err());
        assert!(Chicago::new(&vec![5000; PIXELS]).is_err());
        assert!(
            Chicago::from_print(&Print {
                version: 1,
                background: vec![3000; PIXELS],
                calibration: vec![0; 16],
                gallery: vec![0; 16]
            })
            .is_err()
        );
    }
}
