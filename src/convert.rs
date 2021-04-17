use anyhow::{anyhow, Result};
use std::process::Stdio;
use std::{path::Path, process::Command};

pub fn run_imagemagick(path: &Path, scale: &str) -> Result<Vec<u8>> {
    let output = Command::new("convert")
        .arg(path)
        .args(&[
            "-gravity", "center", "-quality", "80", "-resize", scale, "PNG:-",
        ])
        .stderr(Stdio::inherit())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("Could not execute ImageMagick"));
    }

    Ok(output.stdout)
}

pub fn optimize_png(data: &[u8]) -> Result<Vec<u8>> {
    let options = oxipng::Options::from_preset(3);

    Ok(oxipng::optimize_from_memory(data, &options)?)
}
