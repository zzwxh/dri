use std::{process::Command};

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

fn encode(input: &str) -> String {
    let mut out = Vec::new();
    for b in input.as_bytes() {
        out.push((b & 0x0F) + 0x61);
        out.push(((b & 0xF0) >> 4) + 0x61);
    }
    String::from_utf8(out).unwrap()
}

fn decode(input: &str) -> String {
    let mut out = Vec::new();
    for b in input.as_bytes().chunks(2) {
        out.push((b[0] - 0x61) | (b[1] - 0x61) << 4);
    }
    String::from_utf8(out).unwrap()
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("{:?}", cli);
    Ok(())
}

fn container_run(image: &str, name: &str, port: u16) -> Result<()> {
    run_podman(&[
        "container",
        "run",
        "--rm",
        "--detach",
        "--publish",
        &format!("127.0.0.1:{}:22", port),
        "--name",
        &format!("dri/{}", name),
        &format!("localhost/dri/{}", image),
    ])?;
    Ok(())
}

fn image_list() -> Result<Vec<Image>> {
    let json = run_podman(&["image", "list", "--format", "json"])?;
    let vec = serde_json::from_str(&json)?;
    Ok(vec)
}

#[derive(Debug, Deserialize)]
struct Image {
    #[serde(rename = "Names")]
    names: Vec<String>,
    #[serde(rename = "Size")]
    size: u64,
}

fn run_podman(args: &[&str]) -> Result<String> {
    let out = Command::new("podman").args(args).output()?;
    let stdout = String::from_utf8(out.stdout)?;
    let stderr = String::from_utf8(out.stderr)?;
    ensure!(
        out.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr,
    );
    Ok(stdout)
}
