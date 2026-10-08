// SPDX-License-Identifier: LGPL-3.0-or-later
mod admin;
mod capture;
mod install;
mod mailbox;
mod runtime;
mod security;
mod validation;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use gxfp_core::{image, matcher, template::Template};
use ndarray::Array2;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    version,
    about = "GXFP51B7 fingerprint capture and SDDM authentication"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Authenticate the enrolled account; exit 0 = match, 1 = mismatch, 2 = backend error.
    Verify {
        user: String,
        #[arg(long)]
        diagnostic: bool,
    },
    /// Capture a background and fifteen independent fingerprint references.
    Enroll {
        #[arg(long)]
        user: String,
    },
    /// Commission the installed helper and PAM module with live checks.
    Check,
    /// Enable the validated SDDM fingerprint branch.
    Enable,
    /// Remove the fingerprint branch and stop its runtime.
    Disable,
    /// Install a fresh deployment from a reviewed build and private guest bundle.
    Install {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        user: String,
        #[arg(long, default_value = ".")]
        source: PathBuf,
    },
    /// Start the isolated SGX guest as its dedicated service account.
    VmStart,
    /// Wait for the pinned guest and enclave loader to become ready.
    RuntimeReady,
    /// Compare a saved capture with a compatible local template, without device I/O.
    Score {
        #[arg(long)]
        template: PathBuf,
        #[arg(long)]
        source: PathBuf,
    },
    /// Check encrypted capture and decoding, returning aggregate metadata.
    CaptureCheck,
    /// Evaluate mathematical operations from JSON on stdin (offline testing).
    Evaluate,
}

pub fn private_directory() -> PathBuf {
    let path = Path::new(security::ROOT).join("guest-private");
    if path.is_dir() {
        path
    } else {
        Path::new(security::ROOT).join("work/research/sgx-vm")
    }
}
fn template() -> Result<Template> {
    Template::read(security::open_private(
        &Path::new(security::STATE).join("template.npz"),
    )?)
}
fn verify(user: &str, diagnostic: bool) -> Result<bool> {
    security::root()?;
    let config = security::config()?;
    security::verify_account(user, &config)?;
    let template = template()?;
    let references = template.references();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let source = capture::capture(&private_directory())?;
        let pixels = image::decode(&source)?;
        let background = template
            .background
            .as_slice()
            .ok_or_else(|| anyhow::anyhow!("Noncontiguous template"))?;
        let (drop, contrast) = image::quality(&pixels, background)?;
        if diagnostic {
            eprintln!("{}", serde_json::json!({"drop":drop,"contrast":contrast}));
        }
        if drop < 900. || contrast < 40. {
            continue;
        }
        let probe = image::prepare(&pixels, background)?;
        let result = matcher::match_set(&references, &probe)?;
        if diagnostic {
            eprintln!(
                "{}",
                serde_json::json!({"correlation":result.correlation,"threshold":matcher::THRESHOLD})
            );
        }
        return Ok(result.correlation >= matcher::THRESHOLD);
    }
    Ok(false)
}
fn matrix(value: &serde_json::Value) -> Result<Array2<f64>> {
    let rows: Vec<Vec<f64>> = serde_json::from_value(value.clone())?;
    ensure!(
        rows.len() == 56 && rows.iter().all(|r| r.len() == 72),
        "Expected 56x72 image"
    );
    Ok(Array2::from_shape_vec(
        (56, 72),
        rows.into_iter().flatten().collect(),
    )?)
}
fn rows(value: &Array2<f64>) -> Vec<Vec<f64>> {
    value.rows().into_iter().map(|r| r.to_vec()).collect()
}
fn evaluate() -> Result<serde_json::Value> {
    let mut input = String::new();
    io::stdin()
        .take(8 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let v: serde_json::Value = serde_json::from_str(&input)?;
    let pixels = || -> Result<Vec<u16>> { Ok(serde_json::from_value(v["pixels"].clone())?) };
    let background =
        || -> Result<Vec<u16>> { Ok(serde_json::from_value(v["background"].clone())?) };
    Ok(match v["operation"].as_str() {
        Some("prepare") => serde_json::json!(rows(&image::prepare(&pixels()?, &background()?)?)),
        Some("quality") => serde_json::json!(image::quality(&pixels()?, &background()?)?),
        Some("decode") => serde_json::json!(image::decode(&serde_json::from_value::<Vec<u8>>(
            v["source"].clone()
        )?)?),
        Some("affine") => {
            let angle = v["angle"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("Expected integer angle"))?;
            let (a, m) = matcher::affine(
                &matrix(&v["image"])?,
                angle.try_into()?,
                v["sx"].as_f64().unwrap_or(1.),
                v["sy"].as_f64().unwrap_or(1.),
                v["shear"].as_f64().unwrap_or(0.),
            )?;
            serde_json::json!({"image":rows(&a),"mask":rows(&m)})
        }
        Some("translation") => serde_json::to_value(matcher::translation(
            &matrix(&v["image"])?,
            &matrix(&v["probe"])?,
        )?)?,
        Some("match") => {
            let refs = v["references"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Expected references"))?
                .iter()
                .map(matrix)
                .collect::<Result<Vec<_>>>()?;
            serde_json::to_value(matcher::match_set(&refs, &matrix(&v["probe"])?)?)?
        }
        _ => anyhow::bail!("Unknown mathematical operation"),
    })
}
fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Verify { user, diagnostic } => {
            let result = match verify(&user, diagnostic) {
                Ok(true) => 0,
                Ok(false) => 1,
                Err(error) => {
                    if diagnostic {
                        eprintln!("{error:#}");
                    }
                    2
                }
            };
            std::process::exit(result)
        }
        Commands::Enroll { user } => admin::enroll(&user)?,
        Commands::Check => admin::check()?,
        Commands::Enable => admin::enable()?,
        Commands::Disable => admin::disable()?,
        Commands::Install {
            bundle,
            user,
            source,
        } => install::install(&bundle, &user, &source)?,
        Commands::VmStart => runtime::start()?,
        Commands::RuntimeReady => runtime::ready()?,
        Commands::Evaluate => println!("{}", evaluate()?),
        Commands::Score { template, source } => {
            let t = Template::read(fs::File::open(template)?)?;
            let p = image::decode(&fs::read(source)?)?;
            let background = t
                .background
                .as_slice()
                .ok_or_else(|| anyhow::anyhow!("Noncontiguous template"))?;
            let (drop, contrast) = image::quality(&p, background)?;
            let prepared = image::prepare(&p, background)?;
            let result = matcher::match_set(&t.references(), &prepared)?;
            println!(
                "{}",
                serde_json::json!({"drop":drop,"contrast":contrast,"match":result,"accepted":drop>=900. && contrast>=40. && result.correlation>=matcher::THRESHOLD})
            );
        }
        Commands::CaptureCheck => {
            let start = Instant::now();
            let source = capture::capture(&private_directory())?;
            let p = image::decode(&source)?;
            println!(
                "{}",
                serde_json::json!({"source_bytes":source.len(),"pixels":p.len(),"elapsed_seconds":start.elapsed().as_secs_f64(),"integrity":"verified"})
            );
        }
    }
    Ok(())
}
fn main() {
    let _ = nix::sys::resource::setrlimit(nix::sys::resource::Resource::RLIMIT_CORE, 0, 0);
    if let Err(error) = run(Cli::parse().command) {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}
