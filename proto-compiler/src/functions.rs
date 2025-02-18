use std::{
    env,
    fs::{copy, create_dir_all, read_to_string, remove_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

use crate::constants::{GenerationMode, DEFAULT_TENDERDASH_COMMITISH, DEP_PROTOC_VERSION};

/// Check out a specific commitish of the tenderdash repository.
///
/// As this tool is mainly used by build.rs script, we rely
/// on cargo to decide wherther or not to call it. It means
/// we will not be called too frequently, so the fetch will
/// not happen too often.
pub fn fetch_commitish(tenderdash_dir: &Path, cache_dir: &Path, url: &str, commitish: &str) {
    let url = format!("{url}/archive/{commitish}.zip");

    println!(
        "  [info] => Downloading and extracting {} into {}",
        url,
        tenderdash_dir.to_string_lossy()
    );

    // ensure cache dir exists
    if !cache_dir.is_dir() {
        std::fs::create_dir_all(cache_dir).expect("cannot create cache directory");
    }

    let archive_file = cache_dir.join(format!("tenderdash-{}.zip", commitish));
    // Unzip Tenderdash sources to tmpdir and move to target/tenderdash
    let tmpdir = tempfile::tempdir().expect("cannot create temporary dir to extract archive");
    download_and_unzip(&url, archive_file.as_path(), tmpdir.path());

    // Downloaded zip contains subdirectory like tenderdash-0.12.0-dev.2. We need to
    // move its contents to target/tederdash, so that we get correct paths like
    // target/tenderdash/version/version.go
    let src_dir = find_subdir(tmpdir.path(), "tenderdash-");

    let options = fs_extra::dir::CopyOptions::new().content_only(true);

    fs_extra::dir::create(tenderdash_dir, true).expect("cannot create destination directory");
    fs_extra::dir::move_dir(src_dir, tenderdash_dir, &options)
        .expect("cannot move tenderdash directory");
}

/// Download file from URL and unzip it to `dest_dir`
fn download_and_unzip(url: &str, archive_file: &Path, dest_dir: &Path) {
    const RETRIES: usize = 2;

    for retry in 1..=RETRIES {
        println!(
            "    [info] => Download and extract tenderdash sources, attempt {}/{}",
            retry, RETRIES
        );

        if !archive_file.is_file() {
            println!("      [info] => Downloading {}", url);
            download(url, archive_file)
                .unwrap_or_else(|e| println!(" [error] => Cannot download archive: {:?}", e));
        } else {
            println!(
                "      [info] => Archive file {} already exists, skipping download",
                archive_file.display()
            );
        }

        println!(
            "      [info] => Extracting downloaded archive {}",
            archive_file.display()
        );
        match unzip(archive_file, dest_dir) {
            Ok(_) => break,
            Err(e) => {
                println!(
                    "        [error] => Cannot unzip archive: {}: {:?}",
                    archive_file.display(),
                    e
                );
            },
        }

        // remove invalid file
        std::fs::remove_file(archive_file)
            .unwrap_or_else(|_| println!("      [warn] => Cannot remove file: {:?}", archive_file));
    }

    println!(
        "      [info] => Extracted tenderdash sources to {}",
        dest_dir.display()
    );
}

/// Download file from URL
fn download(url: &str, archive_file: &Path) -> Result<(), String> {
    let mut file =
        File::create(archive_file).map_err(|e| format!("cannot create file: {:?}", e))?;
    let rb = ureq::get(url)
        .call()
        .map_err(|e| format!("cannot download archive from: {}: {:?}", url, e))?;
    let mut body = rb.into_body();
    let mut reader = body.as_reader();
    std::io::copy(&mut reader, &mut file).map_err(|e| {
        format!(
            "cannot save downloaded data to: {:?}: {:?}",
            archive_file, e
        )
    })?;

    file.flush()
        .map_err(|e| format!("cannot flush downloaded file: {:?}: {:?}", archive_file, e))
}

// Unzip archive; when return false, it means that the archive file does not
// exist or is corrupted and should be downloaded again
fn unzip(archive_file: &Path, dest_dir: &Path) -> Result<(), String> {
    if !archive_file.is_file() {
        // no archive file, so we request another download
        return Err("archive file does not exist".to_string());
    }
    let file = File::open(archive_file).expect("cannot open downloaded zip");
    let mut archive =
        zip::ZipArchive::new(&file).map_err(|e| format!("cannot open zip archive: {:?}", e))?;

    archive
        .extract(dest_dir)
        .map_err(|e| format!("cannot extract archive: {:?}", e))?;

    Ok(())
}

/// Find a subdirectory of a parent path which has provided name prefix
fn find_subdir(parent: &Path, name_prefix: &str) -> PathBuf {
    let dir_content = fs_extra::dir::get_dir_content(parent).expect("cannot ls tmp dir");
    let mut src_dir = String::new();
    for directory in dir_content.directories {
        let directory = Path::new(&directory)
            .file_name()
            .expect("cannot extract dir name");

        if directory.to_string_lossy().starts_with(name_prefix) {
            src_dir = directory.to_string_lossy().into();
            break;
        };
    }
    if src_dir.is_empty() {
        panic!("cannot find extracted Tenderdash sources")
    }
    parent.join(src_dir)
}

/// Copy generated files to target folder
pub fn copy_files(src_dir: &Path, target_dir: &Path) {
    // Remove old compiled files
    remove_dir_all(target_dir).unwrap_or_default();
    create_dir_all(target_dir).unwrap();

    // Copy new compiled files (prost does not use folder structures)
    let errors = WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            copy(
                e.path(),
                std::path::Path::new(&format!(
                    "{}/{}",
                    &target_dir.display(),
                    &e.file_name().to_os_string().to_str().unwrap()
                )),
            )
        })
        .filter_map(|e| e.err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for e in errors {
            println!("[error] => Error while copying compiled file: {e}");
        }
        panic!("[error] => Aborted.");
    }
}

/// Walk through the list of directories and gather all *.proto files
pub fn find_proto_files(proto_paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut protos: Vec<PathBuf> = vec![];
    for proto_path in &proto_paths {
        protos.append(
            &mut WalkDir::new(proto_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file()
                        && e.path().extension().is_some()
                        && e.path().extension().unwrap() == "proto"
                })
                .map(|e| e.into_path())
                .collect(),
        );
    }
    protos
}

pub fn abci_version<T: AsRef<Path>>(dir: T) -> String {
    let mut file_path = dir.as_ref().to_path_buf();
    file_path.push("version/version.go");

    let contents = read_to_string(&file_path).expect("cannot read version/version.go");
    use regex::Regex;

    let re = Regex::new(r##"(?m)^\s+ABCISemVer\s*=\s*"([^"]+)"\s+*$"##).unwrap();
    let captures = re
        .captures(&contents)
        .expect("cannot find ABCISemVer in version/version.go");

    captures
        .get(1)
        .expect("ABCISemVer not found in version/version.go")
        .as_str()
        .to_string()
}

pub fn tenderdash_version<T: AsRef<Path>>(dir: T) -> String {
    let mut file_path = dir.as_ref().to_path_buf();
    file_path.push("version/version.go");

    let contents = read_to_string(&file_path).expect("cannot read version/version.go");
    use regex::Regex;

    let re = Regex::new(r##"(?m)^\s+TMVersionDefault\s*=\s*"([^"]+)"\s+*$"##).unwrap();
    let captures = re
        .captures(&contents)
        .expect("cannot find TMVersionDefault in version/version.go");

    captures
        .get(1)
        .expect("TMVersionDefault not found in version/version.go")
        .as_str()
        .to_string()
}

/// Create tenderdash.rs with library information
pub fn generate_tenderdash_lib(
    prost_dir: &Path,
    tenderdash_lib_target: &Path,
    abci_ver: &str,
    td_ver: &str,
    mode: &GenerationMode,
) {
    let mut file_names = WalkDir::new(prost_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.file_name().to_str().unwrap().starts_with("tendermint.")
                && e.file_name().to_str().unwrap().ends_with(".rs")
        })
        .map(|d| d.file_name().to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    file_names.sort();

    let mut content =
        String::from("//! Tenderdash-proto auto-generated sub-modules for Tenderdash\n");
    let tab = "    ".to_string();

    for file_name in file_names {
        let parts: Vec<_> = file_name
            .strip_prefix("tendermint.")
            .unwrap()
            .strip_suffix(".rs")
            .unwrap()
            .split('.')
            .rev()
            .collect();

        let mut tab_count = parts.len();

        let mut inner_content = format!("{}include!(\"./{}\");", tab.repeat(tab_count), file_name);

        for part in parts {
            tab_count -= 1;
            let tabs = tab.repeat(tab_count);
            //{tabs} pub mod {part} {
            //{inner_content}
            //{tabs} }
            inner_content = format!("{tabs}pub mod {part} {{\n{inner_content}\n{tabs}}}");
        }

        content = format!("{content}\n{inner_content}\n");
    }

    // Add meta
    content = format!(
        "{}
pub mod meta {{
    pub const REPOSITORY: &str = \"{}\";
    pub const COMMITISH: &str = \"{}\";
    /// Semantic version of ABCI protocol
    pub const ABCI_VERSION: &str = \"{}\";
    /// Version of Tenderdash server used to generate protobuf configs
    pub const TENDERDASH_VERSION: &str = \"{}\";
    /// Module generation mode
    pub const TENDERDASH_MODULE_MODE: &str = \"{}\";
}}
",
        content,
        crate::constants::TENDERDASH_REPO,
        tenderdash_commitish(),
        abci_ver,
        td_ver,
        mode,
    );

    let mut file =
        File::create(tenderdash_lib_target).expect("tenderdash library file create failed");
    file.write_all(content.as_bytes())
        .expect("tenderdash library file write failed");
}

pub(crate) fn tenderdash_commitish() -> String {
    match env::var("TENDERDASH_COMMITISH") {
        Ok(v) => v,
        Err(_) => DEFAULT_TENDERDASH_COMMITISH.to_string(),
    }
}

/// Save the commitish of last successful download to a file in a state file,
/// located in the `dir` directory and named `download.state`.
pub(crate) fn save_state(dir: &Path, commitish: &str) {
    let state_file = PathBuf::from(&dir).join("download.state");

    std::fs::write(&state_file, commitish)
        .map_err(|e| {
            println!(
                "[warn] => Failed to write download.state file {}: {}",
                state_file.display(),
                e
            );
        })
        .ok();
}

/// Check if the state file contains the same commitish as the one we are trying
/// to download. State file should be located in the `dir` and named
/// `download.state`
pub(crate) fn check_state(dir: &Path, commitish: &str) -> bool {
    let state_file = PathBuf::from(&dir).join("download.state");

    let expected = commitish.to_string();

    match read_to_string(state_file) {
        Ok(content) => {
            println!("[info] => Detected Tenderdash version: {}.", content);
            content == expected
        },
        Err(_) => false,
    }
}

/// Check if all dependencies are met
pub(crate) fn check_deps() -> Result<(), String> {
    dep_protoc(DEP_PROTOC_VERSION).map(|_| ())
}

/// Check if protoc is installed and has the required version
fn dep_protoc(expected_version: f32) -> Result<f32, String> {
    let protoc = prost_build::protoc_from_env();

    // Run `protoc --version` and capture the output
    let output = Command::new(protoc)
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to run: {}", e))?;

    // Convert the output to a string
    let out = output.stdout;
    let version_output = String::from_utf8(out.clone())
        .map_err(|e| format!("output {:?} is not utf8 string: {}", out, e))?;

    // Extract the version number from string like `libprotoc 25.1`
    let version: f32 = version_output
        .split_whitespace()
        .last()
        .unwrap()
        .parse()
        .map_err(|e| format!("failed to parse protoc version {}: {}", version_output, e))?;

    if version < expected_version {
        Err(format!(
            "protoc version must be {} or higher, but found {}; please upgrade: https://github.com/protocolbuffers/protobuf/releases/",
            expected_version, version
        ))
    } else {
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protoc_dep() {
        let expected_versions = vec![(10.1, true), (DEP_PROTOC_VERSION, true), (90.5, false)];
        for expect in expected_versions {
            assert_eq!(dep_protoc(expect.0).is_ok(), expect.1);
        }
    }
}
