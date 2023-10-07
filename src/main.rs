use std::process::Command;

use anyhow::{ensure, Result};
use serde::Deserialize;

fn main() -> Result<()> {
    let images = list_image()?;
    println!("{:?}", images);
    Ok(())
}

fn list_image() -> Result<Vec<Image>> {
    let json = run_podman(&["image", "list", "--format", "json"])?;
    let vec = serde_json::from_str(&json)?;
    Ok(vec)
}

#[derive(Debug, Deserialize)]
struct Image {
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Names")]
    names: Vec<String>,
}

fn run_podman(args: &[&str]) -> Result<String> {
    let out = Command::new("podman").args(args).output()?;
    ensure!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    Ok(stdout)
}
