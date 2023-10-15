use std::process::Command;

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    New { name: String, port: u16 },
    Run { name: String, port: u16 },
    Stop { name: String },
    Rm { name: String },
    Ls {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::New { name, port } => {
            run_podman(&[
                "container",
                "run",
                "--rm",
                "--detach",
                "--publish",
                &encode_port(port),
                "--name",
                &encode_container_name(&name),
                "localhost/drix/default:latest",
            ])?;
        }
        Cmd::Run { name, port } => {
            run_podman(&[
                "container",
                "run",
                "--rm",
                "--detach",
                "--publish",
                &encode_port(port),
                "--name",
                &encode_container_name(&name),
                &encode_image_name(&name),
            ])?;
        }
        Cmd::Stop { name } => {
            run_podman(&[
                "container",
                "commit",
                "--pause",
                &encode_container_name(&name),
                &encode_image_name(&name),
            ])?;
            run_podman(&["container", "stop", &encode_container_name(&name)])?;
        }
        Cmd::Rm { name } => {
            run_podman(&["image", "rm", &encode_image_name(&name)])?;
        }
        Cmd::Ls {} => {
            container_list()?.iter().for_each(|c| {
                println!("{:?}", c);
            });
            image_list()?.iter().for_each(|i| {
                println!("{:?}", i);
            });
        }
    }
    Ok(())
}

fn encode_port(port: u16) -> String {
    format!("127.0.0.1:{}:22", port)
}

fn encode_container_name(name: &str) -> String {
    format!("dri-{}", encode(name).unwrap())
}

fn parse_container_name(name: &str) -> Option<String> {
    match name.strip_prefix("dri-") {
        Some(name) => decode(name).ok(),
        None => None,
    }
}

fn encode_image_name(name: &str) -> String {
    format!("localhost/dri/{}:latest", encode(name).unwrap())
}

fn parse_image_name(name: &str) -> Option<String> {
    match name.strip_prefix("localhost/dri/") {
        Some(name) => match name.strip_suffix(":latest") {
            Some(name) => decode(name).ok(),
            None => None,
        },
        None => None,
    }
}

fn container_list() -> Result<Vec<Container>> {
    let json = run_podman(&["container", "list", "--size", "--format", "json"])?;
    let raw_vec: Vec<RawContainer> = serde_json::from_str(&json)?;
    let vec = raw_vec
        .into_iter()
        .filter_map(|raw| {
            parse_container_name(&raw.names[0]).map(|name| Container {
                name,
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
            parse_image_name(&raw.names[0]).map(|name| Image {
                name,
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
        "\nstdout:\n{}\nstderr:\n{}\n",
        stdout,
        stderr,
    );
    Ok(stdout)
}

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
