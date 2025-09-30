use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use indicatif::{ProgressBar, ProgressStyle};

fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(ConfigError::Help(text)) => {
            println!("{}", text);
            return;
        }
        Err(ConfigError::Message(err)) => {
            eprintln!("Error: {}", err);
            eprintln!();
            eprintln!("{}", Config::usage());
            process::exit(1);
        }
    };

    if let Err(err) = run(&config) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run(config: &Config) -> Result<(), String> {
    let collected = collect_files(&config.root, config.source_extension());

    for warning in &collected.warnings {
        eprintln!("{}", warning);
    }

    if collected.files.is_empty() {
        println!(
            "No .{} files found under {}",
            config.source_extension(),
            config.root.display()
        );
        return Ok(());
    }

    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] {wide_bar:.cyan/blue} {pos}/{len} {msg}",
    )
    .map_err(|err| err.to_string())?;

    let progress = ProgressBar::new(collected.files.len() as u64);
    progress.set_style(style);

    let mut converted = 0usize;
    let mut skipped_existing = 0usize;
    let mut failed: Vec<(PathBuf, String)> = Vec::new();

    for path in &collected.files {
        let display_path = display_relative(&config.root, path);
        progress.set_message(display_path.clone());

        let target = path.with_extension(config.target_extension());

        if target.exists() {
            skipped_existing += 1;
            progress.println(format!(
                "Skipping {} ({} already exists)",
                display_path,
                display_relative(&config.root, &target)
            ));
            progress.inc(1);
            continue;
        }

        match fs::rename(path, &target) {
            Ok(()) => {
                converted += 1;
            }
            Err(err) => {
                let error_text = err.to_string();
                failed.push((path.clone(), error_text.clone()));
                progress.println(format!("Failed to rename {}: {}", display_path, error_text));
            }
        }

        progress.inc(1);
    }

    progress.finish_with_message("renaming complete");

    println!(
        "Converted {} file{} (skipped: {}, failed: {})",
        converted,
        if converted == 1 { "" } else { "s" },
        skipped_existing,
        failed.len()
    );

    if !failed.is_empty() {
        for (path, err) in failed {
            eprintln!(
                "Could not rename {}: {}",
                display_relative(&config.root, &path),
                err
            );
        }
    }

    Ok(())
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn collect_files(root: &Path, extension: &str) -> CollectedFiles {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    while let Some(path) = stack.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(err) => {
                warnings.push(format!("Skipping {}: {}", path.display(), err));
                continue;
            }
        };

        if metadata.is_dir() {
            match fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => stack.push(entry.path()),
                            Err(err) => warnings.push(format!(
                                "Skipping entry in {}: {}",
                                path.display(),
                                err
                            )),
                        }
                    }
                }
                Err(err) => {
                    warnings.push(format!("Skipping directory {}: {}", path.display(), err))
                }
            }
        } else if metadata.is_file() && has_extension(&path, extension) {
            files.push(path);
        } else if metadata.file_type().is_symlink() {
            match fs::metadata(&path) {
                Ok(target_meta) => {
                    if target_meta.is_file() && has_extension(&path, extension) {
                        files.push(path);
                    }
                }
                Err(err) => warnings.push(format!("Skipping symlink {}: {}", path.display(), err)),
            }
        }
    }

    files.sort();

    CollectedFiles { files, warnings }
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

struct Config {
    root: PathBuf,
    invert: bool,
}

enum ConfigError {
    Message(String),
    Help(String),
}

impl Config {
    fn from_env() -> Result<Self, ConfigError> {
        let mut invert = false;
        let mut root: Option<PathBuf> = None;

        for arg in env::args().skip(1) {
            match arg.as_str() {
                "--help" | "-h" => {
                    return Err(ConfigError::Help(Self::usage().to_string()));
                }
                "--invert" => {
                    invert = true;
                }
                other => {
                    if root.is_some() {
                        return Err(ConfigError::Message(format!(
                            "Unexpected argument: {}",
                            other
                        )));
                    }
                    root = Some(PathBuf::from(other));
                }
            }
        }

        let cwd = env::current_dir().map_err(|err| {
            ConfigError::Message(format!("Failed to determine current directory: {}", err))
        })?;

        let root = match root {
            Some(path) => {
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            }
            None => cwd,
        };

        let metadata = fs::metadata(&root).map_err(|err| {
            ConfigError::Message(format!("{} is not accessible: {}", root.display(), err))
        })?;

        if !metadata.is_dir() {
            return Err(ConfigError::Message(format!(
                "{} is not a directory",
                root.display()
            )));
        }

        let resolved = root.canonicalize().map_err(|err| {
            ConfigError::Message(format!("Failed to resolve {}: {}", root.display(), err))
        })?;

        Ok(Self {
            root: resolved,
            invert,
        })
    }

    fn usage() -> &'static str {
        "Usage: r3dy [--invert] [path]\n\nRenames .NEV files to .R3D (or vice versa with --invert) within the given path."
    }

    fn source_extension(&self) -> &'static str {
        if self.invert { "R3D" } else { "NEV" }
    }

    fn target_extension(&self) -> &'static str {
        if self.invert { "NEV" } else { "R3D" }
    }
}

struct CollectedFiles {
    files: Vec<PathBuf>,
    warnings: Vec<String>,
}
