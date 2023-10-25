use std::process::Command;

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

const DOCKERFILE: &str = r#"
FROM docker.io/fedora
RUN dnf -y upgrade
RUN dnf -y install openssh-server passwd 
RUN echo '123456' | passwd --stdin root && echo 'PermitRootLogin yes' >> /etc/ssh/sshd_config && ssh-keygen -A
CMD /usr/sbin/sshd -D
"#;

const BUILD_COMMAND: &str = r#"
podman build --tag localhost/drix/default:latest .
"#;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Print command to build the default image
    Build {},
    /// Lunch a container from the default image
    New { name: String, port: u16 },
    /// Lunch a container from an existing image
    Run { name: String, port: u16 },
    /// List all containers and images
    List {},
    /// Stop a container
    Stop { name: String },
    /// Stop a container without saving
    Kill { name: String },
    /// Remove an image
    Remove { name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Build {} => {
            println!("Dockerfile: {}", DOCKERFILE);
            println!("Build Command: {}", BUILD_COMMAND);
        }
        Cmd::New { name, port } => {
            run_podman(&[
                "container",
                "run",
                "--rm",
                "--detach",
                "--pull",
                "never",
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
                "--pull",
                "never",
                "--publish",
                &encode_port(port),
                "--name",
                &encode_container_name(&name),
                &encode_image_name(&name),
            ])?;
        }
        Cmd::List {} => {
            container_list()?.iter().for_each(|c| {
                println!("{:?}", c);
            });
            image_list()?.iter().for_each(|i| {
                println!("{:?}", i);
            });
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
        Cmd::Kill { name } => {
            run_podman(&["container", "kill", &encode_container_name(&name)])?;
        }
        Cmd::Remove { name } => {
            run_podman(&["image", "rm", &encode_image_name(&name)])?;
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
                size: raw.size.root_fs_size as f64 / 1024.0 / 1024.0,
                port: raw.ports[0].host_port,
            })
        })
        .collect();
    Ok(vec)
}

#[allow(unused)]
#[derive(Debug)]
struct Container {
    name: String,
    size: f64,
    port: u16,
}

#[derive(Deserialize)]
struct RawContainer {
    #[serde(rename = "Names")]
    names: Vec<String>,
    #[serde(rename = "Size")]
    size: RawContainerSize,
    #[serde(rename = "Ports")]
    ports: Vec<RawPortMap>,
}

#[derive(Deserialize)]
struct RawContainerSize {
    #[serde(rename = "rootFsSize")]
    root_fs_size: u64,
}

#[derive(Deserialize)]
struct RawPortMap {
    #[serde(rename = "host_port")]
    host_port: u16,
}

fn image_list() -> Result<Vec<Image>> {
    let json = run_podman(&["image", "list", "--format", "json"])?;
    let raw_vec: Vec<RawImage> = serde_json::from_str(&json)?;
    let vec = raw_vec
        .into_iter()
        .filter_map(|raw| {
            parse_image_name(&raw.names[0]).map(|name| Image {
                name,
                size: raw.size as f64 / 1024.0 / 1024.0,
            })
        })
        .collect();
    Ok(vec)
}

#[allow(unused)]
#[derive(Debug)]
struct Image {
    name: String,
    size: f64,
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
