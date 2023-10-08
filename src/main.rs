use std::process::Command;

use anyhow::{ensure, Ok, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

fn encode(input: &str) -> Result<String> {
    ensure!(input.len() <= 20);
    let mut out = Vec::new();
    for b in input.as_bytes() {
        out.push((b & 0x0F) + 0x61);
        out.push(((b & 0xF0) >> 4) + 0x61);
    }
    Ok(String::from_utf8(out).unwrap())
}

fn decode(input: &str) -> Result<String> {
    ensure!(input.len() <= 40);
    ensure!(input.len() % 2 == 0);
    let mut out = Vec::new();
    for b in input.as_bytes().chunks(2) {
        out.push((b[0] - 0x61) | (b[1] - 0x61) << 4);
    }
    Ok(String::from_utf8(out)?)
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    List,
    Run {
        #[arg(short, long)]
        image: Option<String>,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        port: u16,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("{:?}", cli);
    match cli.cmd {
        Cmd::List => {
            let images: Vec<Image> = image_list()?
                .into_iter()
                .filter(|i| i.name != "default")
                .collect();
            let containers = container_list()?;
            for i in images {
                println!("Image Name:{} Size:{}",decode(&i.name)?,i.size);
            }
            for c in containers {
                println!("Container Name:{} Size:{}",decode(&c.name)?,c.size);
            }
        }
        Cmd::Run { image, name, port } => {
            let image = image.unwrap_or("default".to_string());
            let exists = image_list()?.iter().any(|i| i.name == image);
            ensure!(exists);
            container_run(&image, &name, port)?;
        }
    }
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
        &format!("dri-{}", name),
        &format!("localhost/dri/{}", image),
    ])?;
    Ok(())
}

fn container_list() -> Result<Vec<Container>> {
    let json = run_podman(&["container", "list", "--size", "--format", "json"])?;
    let raw_vec: Vec<RawContainer> = serde_json::from_str(&json)?;
    let vec = raw_vec
        .into_iter()
        .filter_map(|raw| {
            raw.names[0].strip_prefix("dri-").map(|s| Container {
                name: s.to_string(),
                size: raw.size.root_fs_size,
            })
        })
        .collect();
    Ok(vec)
}

#[derive(Debug)]
struct Container {
    name: String,
    size: u64,
}

#[derive(Deserialize)]
struct RawContainer {
    #[serde(rename = "Names")]
    names: Vec<String>,
    #[serde(rename = "Size")]
    size: RawContainerSize,
}

#[derive(Deserialize)]
struct RawContainerSize {
    #[serde(rename = "rootFsSize")]
    root_fs_size: u64,
}

fn image_list() -> Result<Vec<Image>> {
    let json = run_podman(&["image", "list", "--format", "json"])?;
    let raw_vec: Vec<RawImage> = serde_json::from_str(&json)?;
    let vec = raw_vec
        .into_iter()
        .filter_map(|raw| {
            raw.names[0]
                .strip_prefix("localhost/dri/")
                .map(|s| s.strip_suffix(":latest").unwrap())
                .map(|s| Image {
                    name: s.to_string(),
                    size: raw.size,
                })
        })
        .collect();
    Ok(vec)
}

#[derive(Debug)]
struct Image {
    name: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct RawImage {
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
