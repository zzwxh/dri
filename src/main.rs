use std::{net::TcpListener, process::Command};

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

#[test]
fn test() {
    let x = "hello";
    let y = encode(x);
    let z = decode(&y);
    panic!("{}\n{}", y, z);
}

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
    /// list images
    Images,
    /// list containers
    Containers,
    /// new a container
    Run {
        #[arg(long, value_parser=parse_container_name)]
        name: String,
        /// image name
        #[arg(long, default_value = "default", value_parser=parse_image_name)]
        image: String,
        /// ssh port
        #[arg(long, default_value_t = 49222, value_parser=parse_port)]
        port: u16,
    },
    /// stop a container
    Stop {
        name: String,
        /// save before stop
        #[arg(long)]
        save: bool,
    },
    /// remove an image
    Remove { name: String },
}

fn parse_port(raw: &str) -> Result<u16> {
    let port = raw.parse()?;
    ensure!(port >= 49152);
    TcpListener::bind(("127.0.0.1", port))?;
    Ok(port)
}

fn parse_container_name(raw: &str) -> Result<String> {
    let mut name = raw.to_string();
    let vaild = name.chars().all(|c| c.is_ascii_alphanumeric());
    ensure!(vaild);
    name.make_ascii_lowercase();
    ensure!(name != "default");
    Ok(name)
}

fn parse_image_name(raw: &str) -> Result<String> {
    let mut name = raw.to_string();
    Ok(name)
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
