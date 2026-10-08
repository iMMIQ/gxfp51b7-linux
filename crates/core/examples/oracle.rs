// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only JSON interface for independent decoder and quality parity checks.
use anyhow::Result;
use gxfp_core::image;
use std::io::{self, Read};
fn run() -> Result<()> {
    let mut input = String::new();
    io::stdin().take(128 * 1024).read_to_string(&mut input)?;
    let value: serde_json::Value = serde_json::from_str(&input)?;
    let result = match value["operation"].as_str() {
        Some("decode") => serde_json::json!(image::decode(&serde_json::from_value::<Vec<u8>>(
            value["source"].clone()
        )?)?),
        Some("quality") => serde_json::json!(image::quality(
            &serde_json::from_value::<Vec<u16>>(value["pixels"].clone())?,
            &serde_json::from_value::<Vec<u16>>(value["background"].clone())?
        )?),
        _ => anyhow::bail!("Unknown test operation"),
    };
    println!("{result}");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(2);
    }
}
