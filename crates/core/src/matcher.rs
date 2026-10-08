// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Result, ensure};
use ndarray::{Array2, Axis, s};
use ndarray_conv::{FftProcessor, get_fft_processor};
use rustfft::num_complex::Complex;
use serde::Serialize;

pub const THRESHOLD: f64 = 0.86;
const H: usize = 56;
const W: usize = 72;
#[derive(Debug, Clone, Serialize)]
pub struct Match {
    pub correlation: f64,
    pub dx: i32,
    pub dy: i32,
    pub overlap: usize,
    pub angle: i32,
    pub sx: f64,
    pub sy: f64,
    pub shear: f64,
    pub reference_index: usize,
}
impl Default for Match {
    fn default() -> Self {
        Self {
            correlation: -1.,
            dx: 0,
            dy: 0,
            overlap: 0,
            angle: 0,
            sx: 1.,
            sy: 1.,
            shear: 0.,
            reference_index: 0,
        }
    }
}
fn valid(a: &Array2<f64>) -> Result<()> {
    ensure!(
        a.shape() == [H, W] && a.iter().all(|v| v.is_finite()),
        "Invalid matching image"
    );
    Ok(())
}
fn integral(a: &Array2<f64>) -> Array2<f64> {
    let mut result = Array2::zeros((H + 1, W + 1));
    let mut inner = a.clone();
    inner.accumulate_axis_inplace(Axis(0), |&previous, current| *current += previous);
    inner.accumulate_axis_inplace(Axis(1), |&previous, current| *current += previous);
    result.slice_mut(s![1.., 1..]).assign(&inner);
    result
}
fn sum(a: &Array2<f64>, y0: usize, y1: usize, x0: usize, x1: usize) -> f64 {
    a[[y1, x1]] - a[[y0, x1]] - a[[y1, x0]] + a[[y0, x0]]
}
/// Reuse the library's N-dimensional FFT processor and the probe spectra.
/// 128x144 padding covers the complete 111x143 linear correlation support.
struct Correlations<P: FftProcessor<f64, f64>> {
    fft: P,
    probe: Array2<Complex<f64>>,
    probe_squared: Array2<Complex<f64>>,
    integral: Array2<f64>,
    integral_squared: Array2<f64>,
}
fn padded(a: &Array2<f64>, reverse: bool) -> Array2<f64> {
    let mut padded = Array2::zeros((128, 144));
    if reverse {
        padded
            .slice_mut(s![..H, ..W])
            .assign(&a.slice(s![..;-1,..;-1]));
    } else {
        padded.slice_mut(s![..H, ..W]).assign(a);
    }
    padded
}
impl<P: FftProcessor<f64, f64>> Correlations<P> {
    fn new(b: &Array2<f64>, mut fft: P) -> Result<Self> {
        valid(b)?;
        let squared = b.mapv(|v| v * v);
        let probe = fft.forward(&mut padded(b, true), false);
        let probe_squared = fft.forward(&mut padded(&squared, true), false);
        Ok(Self {
            fft,
            probe,
            probe_squared,
            integral: integral(b),
            integral_squared: integral(&squared),
        })
    }
    fn spectrum(&mut self, a: &Array2<f64>) -> Array2<Complex<f64>> {
        self.fft.forward(&mut padded(a, false), false)
    }
    fn correlate(&mut self, a: &Array2<Complex<f64>>, squared: bool) -> Array2<f64> {
        let b = if squared {
            &self.probe_squared
        } else {
            &self.probe
        };
        let mut product = a * b;
        self.fft.backward(&mut product, false)
    }
}
fn score(
    a: &Array2<f64>,
    mask: Option<&Array2<f64>>,
    fft: &mut Correlations<impl FftProcessor<f64, f64>>,
) -> Result<Match> {
    valid(a)?;
    let masked;
    let a = if let Some(mask) = mask {
        valid(mask)?;
        masked = a * mask;
        &masked
    } else {
        a
    };
    let ia = integral(a);
    let ia2 = integral(&a.mapv(|v| v * v));
    let spectrum = fft.spectrum(a);
    let cross = fft.correlate(&spectrum, false);
    let masks = if let Some(mask) = mask {
        let spectrum = fft.spectrum(mask);
        Some((
            integral(mask),
            fft.correlate(&spectrum, false),
            fft.correlate(&spectrum, true),
        ))
    } else {
        None
    };
    let mut best = Match::default();
    // Preserve NumPy's row-major argmax order, including tie handling.
    for dy in -24_i32..=24 {
        for dx in -30_i32..=30 {
            let y0 = dy.max(0) as usize;
            let y1 = (H as i32 + dy).min(H as i32) as usize;
            let x0 = dx.max(0) as usize;
            let x1 = (W as i32 + dx).min(W as i32) as usize;
            let by0 = (-dy).max(0) as usize;
            let by1 = (H as i32 - dy).min(H as i32) as usize;
            let bx0 = (-dx).max(0) as usize;
            let bx1 = (W as i32 - dx).min(W as i32) as usize;
            let index = [(H as i32 - 1 + dy) as usize, (W as i32 - 1 + dx) as usize];
            let (area, sb, sb2) = if let Some((im, mb, mb2)) = &masks {
                (sum(im, y0, y1, x0, x1), mb[index], mb2[index])
            } else {
                (
                    ((y1 - y0) * (x1 - x0)) as f64,
                    sum(&fft.integral, by0, by1, bx0, bx1),
                    sum(&fft.integral_squared, by0, by1, bx0, bx1),
                )
            };
            if area < 1600. {
                continue;
            }
            let sa = sum(&ia, y0, y1, x0, x1);
            let va = (sum(&ia2, y0, y1, x0, x1) - sa * sa / area).max(0.);
            let vb = (sb2 - sb * sb / area).max(0.);
            let denominator = (va * vb).sqrt();
            if denominator <= 1e-9 {
                continue;
            }
            let correlation = ((cross[index] - sa * sb / area) / denominator).clamp(-1., 1.);
            if correlation > best.correlation {
                best = Match {
                    correlation,
                    dx,
                    dy,
                    overlap: area as usize,
                    ..Match::default()
                };
            }
        }
    }
    Ok(best)
}

/// Apply the existing output-to-input affine policy using library interpolation.
/// Out-of-sensor samples receive constant zero values and an invalid mask.
pub fn affine(
    a: &Array2<f64>,
    angle: i32,
    sx: f64,
    sy: f64,
    shear: f64,
) -> Result<(Array2<f64>, Array2<f64>)> {
    valid(a)?;
    ensure!(
        (-24..=24).contains(&angle)
            && (0.94..=1.06).contains(&sx)
            && (0.94..=1.06).contains(&sy)
            && (-0.03..=0.03).contains(&shear),
        "Unbounded deformation"
    );
    let (sin, cos) = (angle as f64).to_radians().sin_cos();
    let m = [
        [cos * sy, cos * shear + sin * sx],
        [-sin * sy, -sin * shear + cos * sx],
    ];
    let center = [(H - 1) as f64 / 2., (W - 1) as f64 / 2.];
    let offset = [
        center[0] - m[0][0] * center[0] - m[0][1] * center[1],
        center[1] - m[1][0] * center[0] - m[1][1] * center[1],
    ];
    let mut ys = Vec::new();
    let mut xs = Vec::new();
    let mut indices = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let iy = m[0][0] * y as f64 + m[0][1] * x as f64 + offset[0];
            let ix = m[1][0] * y as f64 + m[1][1] * x as f64 + offset[1];
            if (0.0..=(H - 1) as f64).contains(&iy) && (0.0..=(W - 1) as f64).contains(&ix) {
                ys.push(iy);
                xs.push(ix);
                indices.push((y, x));
            }
        }
    }
    let contiguous = a.as_standard_layout();
    let mut values = vec![0.; indices.len()];
    interpn::multilinear::regular::interpn(
        &[H, W],
        &[0., 0.],
        &[1., 1.],
        contiguous
            .as_slice()
            .ok_or_else(|| anyhow::anyhow!("Noncontiguous image"))?,
        &[&ys, &xs],
        &mut values,
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    let mut output = Array2::zeros((H, W));
    let mut mask = Array2::zeros((H, W));
    for ((y, x), value) in indices.into_iter().zip(values) {
        output[[y, x]] = value;
        mask[[y, x]] = 1.;
    }
    Ok((output, mask))
}

pub fn translation(a: &Array2<f64>, b: &Array2<f64>) -> Result<Match> {
    score(
        a,
        None,
        &mut Correlations::new(b, get_fft_processor::<f64, f64>())?,
    )
}
pub fn match_set(references: &[Array2<f64>], probe: &Array2<f64>) -> Result<Match> {
    ensure!(
        !references.is_empty() && references.len() <= 15,
        "Expected 1..15 references"
    );
    valid(probe)?;
    let mut fft = Correlations::new(probe, get_fft_processor::<f64, f64>())?;
    let mut coarse = Vec::with_capacity(references.len());
    for (i, reference) in references.iter().enumerate() {
        let mut best = score(reference, None, &mut fft)?;
        best.reference_index = i;
        for angle in (-24..=24).step_by(3) {
            if angle == 0 {
                continue;
            }
            let (transformed, mask) = affine(reference, angle, 1., 1., 0.)?;
            let candidate = score(&transformed, Some(&mask), &mut fft)?;
            if candidate.correlation > best.correlation {
                best = Match {
                    angle,
                    reference_index: i,
                    ..candidate
                };
            }
        }
        coarse.push(best);
    }
    let mut best = coarse[0].clone();
    for candidate in &coarse[1..] {
        if candidate.correlation > best.correlation {
            best = candidate.clone();
        }
    }
    coarse.sort_by(|a, b| b.correlation.total_cmp(&a.correlation));
    for candidate in coarse.iter().take(3) {
        for angle in (-24).max(candidate.angle - 2)..=24.min(candidate.angle + 2) {
            for sx in [0.94, 0.97, 1., 1.03, 1.06] {
                for sy in [0.94, 0.97, 1., 1.03, 1.06] {
                    for shear in [-0.03, 0., 0.03] {
                        let (transformed, mask) =
                            affine(&references[candidate.reference_index], angle, sx, sy, shear)?;
                        let refined = score(&transformed, Some(&mask), &mut fft)?;
                        if refined.correlation > best.correlation {
                            best = Match {
                                angle,
                                sx,
                                sy,
                                shear,
                                reference_index: candidate.reference_index,
                                ..refined
                            };
                        }
                    }
                }
            }
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn texture() -> Array2<f64> {
        Array2::from_shape_fn((H, W), |(y, x)| {
            ((y * 17 + x * 31) as f64).sin() + ((y * 53 + x * 7) as f64).cos()
        })
    }
    #[test]
    fn translation_and_blank() {
        let a = texture();
        let mut b = Array2::zeros((H, W));
        b.slice_mut(s![3.., 5..])
            .assign(&a.slice(s![..H - 3, ..W - 5]));
        let result = translation(&a, &b).unwrap();
        assert!((result.correlation - 1.).abs() < 1e-12);
        assert_eq!((result.dx, result.dy), (-5, -3));
        assert_eq!(
            translation(&Array2::zeros((H, W)), &Array2::zeros((H, W)))
                .unwrap()
                .correlation,
            -1.
        );
    }
    #[test]
    fn rejects_invalid_values_and_transform() {
        let mut a = texture();
        a[[0, 0]] = f64::NAN;
        assert!(translation(&a, &a).is_err());
        assert!(affine(&texture(), 0, 0.5, 1., 0.).is_err());
        assert!(match_set(&[], &texture()).is_err());
    }
    #[test]
    fn identity_affine() {
        let a = texture();
        let (b, mask) = affine(&a, 0, 1., 1., 0.).unwrap();
        assert!(a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-12));
        assert!(mask.iter().all(|&v| v == 1.));
    }
}
