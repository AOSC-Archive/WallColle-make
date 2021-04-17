use anyhow::Result;
use log::error;
use std::io::prelude::*;
use std::io::{BufReader, Read};

pub fn parse_manifest<R: Read>(input: R) -> Result<Vec<(String, usize)>> {
    let buffer = BufReader::new(input);
    let mut result: Vec<(String, usize)> = Vec::new();
    result.reserve(20);
    for line in buffer.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut elem = line.splitn(2, ':');
        let name = elem.next();
        let value = elem.next();
        if name.is_none() || value.is_none() {
            error!("Invalid manifest line: `{}`", line);
            continue;
        }
        let entry = value.unwrap().parse();
        if entry.is_err() {
            error!("Cannot parse `{}` as number", value.unwrap());
            continue;
        }
        result.push((name.unwrap().to_string(), entry.unwrap()));
    }

    Ok(result)
}
