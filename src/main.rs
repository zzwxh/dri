use std::process::Command;

use anyhow::{ensure, Result};
use serde::Deserialize;

fn main() -> Result<()> {
    let images = image_list()?;
    println!("{:?}", images);
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
