// SPDX-License-Identifier: LGPL-3.0-or-later
use anyhow::{Context, Result, ensure};
use gxfp_backends::{chicago::Chicago, sift};
use gxfp_core::{image, matcher};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
#[derive(Deserialize)]
struct Probe {
    name: String,
    source: PathBuf,
    enrolled: bool,
}
#[derive(Deserialize)]
struct Manifest {
    background: PathBuf,
    references: Vec<PathBuf>,
    probes: Vec<Probe>,
}
fn pixels(base: &Path, path: &Path) -> Result<Vec<u16>> {
    image::decode(&fs::read(base.join(path)).with_context(|| format!("Read {}", path.display()))?)
}
pub fn run(path: &Path) -> Result<()> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        (1..=15).contains(&manifest.references.len())
            && (1..=10000).contains(&manifest.probes.len()),
        "Bounded reference/probe set required"
    );
    let base = path.parent().unwrap_or(Path::new("."));
    let background = pixels(base, &manifest.background)?;
    let mut chicago = Chicago::new(&background)?;
    let mut enrollment = Vec::new();
    let mut affine = Vec::new();
    let mut local = Vec::new();
    for source in &manifest.references {
        let raw = pixels(base, source)?;
        enrollment.push(chicago.enroll(&raw)?);
        affine.push(image::prepare(&raw, &background)?);
        local.push(sift::extract(&raw, &background)?);
    }
    let print = chicago.print()?;
    let mut records = Vec::new();
    for probe in manifest.probes {
        let raw = pixels(base, &probe.source)?;
        let (drop, contrast) = image::quality(&raw, &background)?;
        let start = Instant::now();
        let ncc = matcher::match_set(&affine, &image::prepare(&raw, &background)?)?;
        let ncc_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let native = Chicago::from_print(&print)?.verify(&raw)?;
        let native_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let features = sift::compare(&local, &sift::extract(&raw, &background)?)?;
        let features_seconds = start.elapsed().as_secs_f64();
        records.push(json!({"name":probe.name,"enrolled":probe.enrolled,
            "affine_ncc":{"correlation":ncc.correlation,"accepted":drop>=900. && contrast>=40. && ncc.correlation>=matcher::THRESHOLD,"seconds":ncc_seconds},
            "chicago":{"evidence":native,"seconds":native_seconds},
            "rootsift":{"evidence":features,"seconds":features_seconds}}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"version":1,"references":affine.len(),
        "chicago_enrollment":enrollment,"probes":records,"scope":"offline comparison; RootSIFT scores require independent policy calibration"}))?
    );
    Ok(())
}
