// SPDX-License-Identifier: LGPL-3.0-or-later
//! SIGFM-inspired local matching using OpenCV's SIFT, nearest-neighbour and
//! RANSAC implementations. Scores are evaluation evidence, not an authentication policy.
use anyhow::{Result, ensure};
use opencv::{
    core::{self, DMatch, KeyPoint, Mat, Point2f, Size, Vector},
    imgproc,
    prelude::*,
};
use serde::Serialize;
use std::collections::BTreeSet;
opencv::opencv_branch_5! { use opencv::{features, geometry}; }
opencv::opencv_branch_4! { use opencv::{features2d as features, calib3d as geometry}; }
pub struct View {
    keys: Vector<KeyPoint>,
    descriptors: Mat,
}
#[derive(Debug, Default, Serialize)]
pub struct Evidence {
    pub keypoints: usize,
    pub best_inliers: usize,
    pub fused_inliers: usize,
    pub contributing_views: usize,
}
pub fn extract(raw: &[u16], background: &[u16]) -> Result<View> {
    ensure!(
        raw.len() == 5120
            && background.len() == 5120
            && raw.iter().chain(background).all(|&v| v <= 4095),
        "Expected twelve-bit 80x64 images"
    );
    let delta: Vec<i32> = background
        .iter()
        .zip(raw)
        .map(|(&b, &p)| i32::from(b) - i32::from(p))
        .collect();
    let min = *delta.iter().min().unwrap();
    let max = *delta.iter().max().unwrap();
    let bytes: Vec<u8> = delta
        .iter()
        .map(|&v| {
            if max == min {
                0
            } else {
                ((v - min) * 255 / (max - min)) as u8
            }
        })
        .collect();
    let image = Mat::from_slice_2d(bytes.as_chunks::<80>().0)?;
    let mut enhanced = Mat::default();
    imgproc::create_clahe(2., Size::new(4, 4))?.apply(&image, &mut enhanced)?;
    let mut keys = Vector::new();
    let mut descriptors = Mat::default();
    features::SIFT::create_def()?.detect_and_compute(
        &enhanced,
        &core::no_array(),
        &mut keys,
        &mut descriptors,
        false,
    )?;
    // RootSIFT: apply the published Hellinger embedding to library descriptors.
    if !descriptors.empty() {
        for row in descriptors
            .data_typed_mut::<f32>()?
            .as_chunks_mut::<128>()
            .0
        {
            let total: f32 = row.iter().map(|v| v.abs()).sum();
            if total > 0. {
                for v in row {
                    *v = (*v / total).sqrt();
                }
            }
        }
    }
    Ok(View { keys, descriptors })
}
pub fn compare(references: &[View], probe: &View) -> Result<Evidence> {
    ensure!(
        !references.is_empty() && references.len() <= 50,
        "Expected 1..50 SIFT views"
    );
    let mut result = Evidence {
        keypoints: probe.keys.len(),
        ..Evidence::default()
    };
    if probe.keys.len() < 3 {
        return Ok(result);
    }
    let matcher = features::BFMatcher::create_def()?;
    let mut fused = BTreeSet::new();
    for view in references {
        if view.keys.len() < 3 {
            continue;
        }
        let mut candidates = Vector::<Vector<DMatch>>::new();
        matcher.knn_train_match_def(&probe.descriptors, &view.descriptors, &mut candidates, 2)?;
        let mut from = Vector::<Point2f>::new();
        let mut to = Vector::<Point2f>::new();
        let mut query_ids = Vec::new();
        let mut used_train = BTreeSet::new();
        for pair in candidates {
            if pair.len() != 2 {
                continue;
            }
            let a = pair.get(0)?;
            let b = pair.get(1)?;
            if a.distance < 0.75 * b.distance && used_train.insert(a.train_idx) {
                from.push(probe.keys.get(a.query_idx as usize)?.pt());
                to.push(view.keys.get(a.train_idx as usize)?.pt());
                query_ids.push(a.query_idx as usize);
            }
        }
        if from.len() < 3 {
            continue;
        }
        let mut inliers = Mat::default();
        let transform = geometry::estimate_affine_partial_2d(
            &from,
            &to,
            &mut inliers,
            geometry::RANSAC,
            3.,
            2000,
            0.99,
            10,
        )?;
        if transform.empty() {
            continue;
        }
        let a = *transform.at_2d::<f64>(0, 0)?;
        let b = *transform.at_2d::<f64>(1, 0)?;
        let scale = a.hypot(b);
        let angle = b.atan2(a).to_degrees();
        if !(0.94..=1.06).contains(&scale) || angle.abs() > 24. {
            continue;
        }
        // Bound centre displacement as in the current matcher, rather than the
        // translation coefficient of a rotation around the upper-left corner.
        let x = a * 39.5 - b * 31.5 + *transform.at_2d::<f64>(0, 2)? - 39.5;
        let y = b * 39.5 + a * 31.5 + *transform.at_2d::<f64>(1, 2)? - 31.5;
        if x.abs() > 30. || y.abs() > 24. {
            continue;
        }
        let mut accepted = BTreeSet::new();
        for (id, &mask) in query_ids.iter().zip(inliers.data_typed::<u8>()?) {
            if mask != 0 {
                let p = probe.keys.get(*id)?.pt();
                // SIFT may return several orientations for one spatial point.
                accepted.insert(((p.x * 4.).round() as i32, (p.y * 4.).round() as i32));
            }
        }
        if accepted.len() >= 3 {
            result.contributing_views += 1;
            result.best_inliers = result.best_inliers.max(accepted.len());
            fused.extend(accepted);
        }
    }
    result.fused_inliers = fused.len();
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blank_has_no_evidence() {
        let blank = extract(&vec![3000; 5120], &vec![3000; 5120]).unwrap();
        let result = compare(std::slice::from_ref(&blank), &blank).unwrap();
        assert_eq!(result.fused_inliers, 0);
        assert!(extract(&[0; 5], &[0; 5]).is_err());
    }
}
