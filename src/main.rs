use anyhow::{anyhow, Result};
use argh::FromArgs;
use fs::File;
use log::*;
use rayon::prelude::*;
use sailfish::TemplateOnce;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::{collections::HashSet, fs};
use std::{io::Write, path::PathBuf};

mod convert;
mod parser;

/// Distribution directories
const DEST_DIRS: &[&str] = &[
    "usr/share/wallpapers",
    "usr/share/backgrounds/xfce",
    "usr/share/background-properties",
    "usr/share/gnome-background-properties",
    "usr/share/mate-background-properties",
];
/// Resolutions for mainline AOSC OS
const MAINLINE_RESOLUTIONS: &[&str] = &[
    "1024x768",
    "1152x768",
    "1280x1024",
    "1280x800",
    "1280x854",
    "1280x960",
    "1366x768",
    "1440x900",
    "1440x960",
    "1600x1200",
    "1600x900",
    "1680x1050",
    "1920x1080",
    "1920x1200",
    "2048x1536",
    "2048x2048",
    "2160x1440",
    "2520x1080",
    "3360x1440",
    "2560x2048",
    "2560x1600",
    "2880x1800",
    "3000x2000",
    "3840x2160",
    "4096x4096",
    "4500x3000",
    "5120x4096",
    "800x600",
];
/// Resolutions for AOSC OS/Retro
const RETRO_RESOLUTIONS: &[&str] = &["800x600", "1280x960", "1600x1200", "1920x1200"];
/// Xfce ratios
const XFCE_RATIOS: &[&str] = &["1-1", "16-10", "16-9", "21-9", "3-2", "4-3", "5-4"];

/// A general purpose wallpaper collection generator
#[derive(FromArgs)]
struct WallColle {
    /// path to the directory containing the wallpaper packs
    #[argh(option)]
    path: String,
    /// path to the output directory
    #[argh(option)]
    dest: String,
    /// pack variant, possible values are: "normal" or "retro"
    #[argh(option)]
    variant: String,
    /// remove the destination directory if it exists
    #[argh(switch)]
    clean: bool
}

enum Variant {
    Normal,
    Retro,
}

#[derive(Deserialize, Serialize, Clone)]
struct WallPaperMeta {
    #[serde(rename = "i")]
    index: usize,
    #[serde(rename = "f")]
    format: String,
    #[serde(rename = "t")]
    title: String,
    #[serde(rename = "l")]
    license: String,
    tags: Vec<String>,
    #[serde(skip)]
    email: String,
    #[serde(skip)]
    artist: String,
    #[serde(skip)]
    src: PathBuf,
    #[serde(skip)]
    dest: String,
    #[serde(skip)]
    entry_name: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct ContributorMeta {
    name: String,
    #[serde(rename = "uname")]
    username: String,
    email: String,
    uri: String,
    src: Option<String>,
    wallpapers: Vec<WallPaperMeta>,
}

#[derive(TemplateOnce)]
#[template(path = "album-gnome.stpl")]
struct AlbumTemplateGTK {
    wallpapers: Vec<WallPaperMeta>,
}

#[derive(TemplateOnce)]
#[template(path = "album-kde.desktop")]
struct AlbumTemplateKDE {
    wallpaper: WallPaperMeta,
}

#[inline]
fn make_dest_dirs<P: AsRef<Path>>(dest: P) -> Result<()> {
    for dir in DEST_DIRS {
        fs::create_dir_all(dest.as_ref().join(dir))?;
    }

    Ok(())
}

#[inline]
fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[inline]
fn normalize_album_name(name: &str) -> String {
    let lower = slug::slugify(name);
    uppercase_first_letter(&lower).replace('-', ".")
}

#[inline]
fn normalize_image_name(album: &str, title: &str, username: &str) -> String {
    let lower = slug::slugify(title);
    let converted = lower
        .split('-')
        .map(|s| uppercase_first_letter(s))
        .collect::<String>();
    format!("{}--{}--{}", album, username, converted)
}

fn write_gtk_config<P: AsRef<Path>>(
    dest: P,
    album: &str,
    wallpapers: Vec<WallPaperMeta>,
) -> Result<()> {
    info!("Writing GTK manifests ...");
    let data = AlbumTemplateGTK { wallpapers }.render_once()?;

    let config_file = format!("{}.xml", normalize_album_name(album));
    let config_file_path = format!("/usr/share/background-properties/{}", &config_file);
    let config = dest.as_ref().join(&config_file_path[1..]);
    let mut f = File::create(&config)?;
    f.write_all(data.as_bytes())?;

    for dir in &DEST_DIRS[3..] {
        symlink(
            &config_file_path,
            dest.as_ref().join(dir).join(&config_file),
        )?;
    }

    Ok(())
}

fn process_single_entry(dest: &Path, entry: &WallPaperMeta, retro: bool) -> Result<()> {
    let entry_name = &entry.entry_name;
    let file_name = format!("{}.{}", entry.index, entry.format);
    let src_path = entry.src.join(&file_name);
    let image_path = &entry.dest;
    let dest_path = dest.join(&image_path[1..]);
    let desktop_path = dest.join(format!(
        "usr/share/wallpapers/{}/metadata.desktop",
        &entry_name
    ));
    let desktop_file = AlbumTemplateKDE {
        wallpaper: entry.clone(),
    }
    .render_once()?;

    fs::create_dir_all(
        dest_path
            .parent()
            .ok_or_else(|| anyhow!("Unable to determine destination directory"))?,
    )?;
    fs::create_dir_all(dest.join(format!(
        "usr/share/wallpapers/{}/contents/images",
        &entry_name
    )))?;

    let mut f = File::create(&desktop_path)?;
    f.write_all(desktop_file.as_bytes())?;

    if !retro {
        info!("Copying: {:?} -> {:?}", src_path, dest_path);
        fs::copy(&src_path, dest_path)?;

        info!("Creating symlinks ...");
        symlink(
            &image_path,
            dest.join(format!(
                "usr/share/wallpapers/{}/screenshot.{}",
                entry_name, entry.format
            )),
        )?;
    }

    for ratio in XFCE_RATIOS {
        symlink(
            &image_path,
            dest.join(format!(
                "usr/share/backgrounds/xfce/{}-{}.{}",
                entry_name, ratio, entry.format
            )),
        )?;
    }

    if retro {
        process_retro(&src_path, dest, &entry_name)?;
    } else {
        process_mainline(image_path, dest, &entry_name, &entry.format)?;
    }

    Ok(())
}

fn process_mainline(image_path: &str, dest: &Path, entry_name: &str, format: &str) -> Result<()> {
    for res in MAINLINE_RESOLUTIONS {
        symlink(
            &image_path,
            dest.join(format!(
                "usr/share/wallpapers/{}/contents/images/{}.{}",
                entry_name, res, format
            )),
        )?;
    }

    Ok(())
}

fn process_retro(src_path: &Path, dest: &Path, entry_name: &str) -> Result<()> {
    RETRO_RESOLUTIONS
        .par_iter()
        .try_for_each(|res| -> Result<()> {
            info!("Processing {} at {}", entry_name, res);
            let filename = format!(
                "usr/share/wallpapers/{}/contents/images/{}.png",
                entry_name, res
            );
            let png = convert::optimize_png(&convert::run_imagemagick(src_path, res)?)?;
            let mut f = File::create(dest.join(filename))?;
            f.write_all(&png)?;

            Ok(())
        })?;

    symlink(
        format!(
            "/usr/share/wallpapers/{}/contents/images/1280x960.png",
            entry_name
        ),
        dest.join(format!(
            "usr/share/wallpapers/{}/screenshot.png",
            entry_name
        )),
    )?;

    Ok(())
}

fn scan_single_artist<P: AsRef<Path>>(
    album: &str,
    path: P,
    selections: &HashSet<usize>,
) -> Result<Vec<WallPaperMeta>> {
    let f = File::open(path.as_ref().join("me.json"))?;
    let artist: ContributorMeta = serde_json::from_reader(f)?;
    let mut results = Vec::new();

    for entry in &artist.wallpapers {
        if !selections.contains(&entry.index) {
            continue;
        }
        results.push(scan_entries(album, entry.clone(), &artist, path.as_ref()));
    }

    Ok(results)
}

fn scan_entries(
    album: &str,
    entry: WallPaperMeta,
    artist: &ContributorMeta,
    artist_path: &Path,
) -> WallPaperMeta {
    let entry_name = normalize_image_name(album, &entry.title, &artist.username);
    let image_path = format!(
        "/usr/share/backgrounds/{}/{}.{}",
        entry_name, entry_name, entry.format
    );
    let mut entry = entry;
    entry.artist = artist.name.clone();
    entry.dest = image_path;
    entry.email = artist.email.clone();
    entry.entry_name = entry_name;
    entry.src = artist_path.to_owned();

    entry
}

fn scan_all_artists(
    lookup: &[(String, HashSet<usize>)],
    pack_root: &Path,
    pack_name: &str,
) -> Result<Vec<WallPaperMeta>> {
    let mut all_data = Vec::new();

    for (artist, selections) in lookup {
        let artist_path = Path::new(pack_root).join("contributors").join(&artist);
        info!("Processing {:?} ...", artist_path);
        let artist_data = scan_single_artist(pack_name, artist_path, selections)?;
        all_data.extend(artist_data);
    }

    Ok(all_data)
}

fn group_by_artist(input: Vec<(String, usize)>) -> Vec<(String, HashSet<usize>)> {
    let mut results = Vec::new();
    let mut submissions = HashSet::new();
    let mut current_artist = String::new();

    for (artist, number) in input {
        if current_artist.is_empty() {
            current_artist = artist;
        } else if current_artist != artist {
            results.push((current_artist, submissions));
            current_artist = artist;
            submissions = HashSet::new();
            submissions.insert(number);
            continue;
        }
        submissions.insert(number);
    }

    results.push((current_artist, submissions));

    results
}

fn main() {
    let args: WallColle = argh::from_env();
    env_logger::init();
    let variant = match args.variant.to_lowercase().as_str() {
        "normal" => Variant::Normal,
        "retro" => Variant::Retro,
        _ => panic!("Unknown variant '{}'", args.variant),
    };
    let is_retro = match variant {
        Variant::Normal => false,
        Variant::Retro => true,
    };
    let dest_path = Path::new(&args.dest);
    if is_retro && which::which("convert").is_err() {
        error!("ImageMagic is not installed!");
        panic!("ImageMagic unavailable!");
    }
    info!(
        "Building {} variant wallpaper pack from '{}' to '{}'",
        args.variant, args.path, args.dest
    );
    if args.clean && dest_path.exists() {
        info!("Purging destination directory...");
        fs::remove_dir_all(dest_path).unwrap();
    }
    let pack_name = Path::new(&args.path)
        .file_name()
        .expect("Failed to get pack name")
        .to_string_lossy();
    info!("Creating directories ...");
    make_dest_dirs(dest_path).unwrap();

    info!("Organizing files ...");
    let pack_file = File::open(dest_path).unwrap();
    let mut pack_data = parser::parse_manifest(pack_file).unwrap();
    pack_data.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let pack_root = dest_path.parent().unwrap().parent().unwrap();

    let lookup = group_by_artist(pack_data);
    let all_data = scan_all_artists(&lookup, pack_root, &pack_name).unwrap();

    all_data
        .par_iter()
        .try_for_each(|entry| -> Result<()> { process_single_entry(dest_path, entry, is_retro) })
        .unwrap();
    write_gtk_config(dest_path, &pack_name, all_data).unwrap();

    info!("Generation complete!");
}
