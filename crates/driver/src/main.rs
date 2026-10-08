// SPDX-License-Identifier: LGPL-3.0-or-later
mod admin;
mod capture;
mod fprint;
mod install;
mod mailbox;
mod runtime;
mod security;
mod validation;

use anyhow::Result;
use clap::{Parser, Subcommand};
use gxfp_core::{calibration, image};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser)]
#[command(
    version,
    about = "GXFP51B7 capture and standard fprintd authentication"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Capture an empty-sensor background and bind the local account.
    Calibrate {
        #[arg(long)]
        user: String,
    },
    /// Validate standard PAM with the enrolled finger and independent controls.
    Check,
    /// Enable the validated standard fprintd branch in SDDM.
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
    /// Check encrypted capture and decoding, returning aggregate metadata.
    CaptureCheck,
    /// Root-only libfprint worker; print bytes travel through private pipes.
    FprintWorker {
        user: String,
        #[arg(long)]
        enroll: bool,
    },
}
pub(crate) fn private_directory() -> PathBuf {
    let path = Path::new(security::ROOT).join("guest-private");
    if path.is_dir() {
        path
    } else {
        // Existing commissioned deployments retain their pinned guest identity.
        Path::new(security::ROOT).join("work/research/sgx-vm")
    }
}
fn background_path() -> PathBuf {
    let current = Path::new(security::STATE).join("background.json");
    if current.exists() {
        current
    } else {
        Path::new(security::STATE).join("template.npz")
    }
}
fn background() -> Result<Vec<u16>> {
    let path = background_path();
    let file = security::open_private(&path)?;
    if path.extension().is_some_and(|ext| ext == "npz") {
        calibration::legacy_background(file)
    } else {
        let pixels = serde_json::from_reader::<_, Vec<u16>>(file.take(128 * 1024))?;
        calibration::validate(&pixels)?;
        Ok(pixels)
    }
}
fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Calibrate { user } => admin::calibrate(&user)?,
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
        Commands::FprintWorker { user, enroll } => fprint::run(&user, enroll)?,
        Commands::CaptureCheck => {
            let start = Instant::now();
            let source = capture::capture(&private_directory())?;
            let pixels = image::decode(&source)?;
            println!(
                "{}",
                serde_json::json!({"source_bytes":source.len(),"pixels":pixels.len(),
                "elapsed_seconds":start.elapsed().as_secs_f64(),"integrity":"verified"})
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
