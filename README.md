<p align="center">
  <img src="docs/red-logo.png" alt="r3dy logo" width="160" />
</p>

# r3dy

Tiny CLI for cinematographers who need to flip Nikon Z8 `.NEV` files over to `.R3D` in a single pass. Point it at a working directory (or let it default to the current one) and enjoy a clean progress bar while your clips get renamed. Add `--invert` when you want to reverse the process.

## Install

```
cargo install r3dy
```

> Already working on the repo? Use `cargo install --path .` to build straight from source.

## Usage

```
r3dy [--invert] [path]
```

- Without arguments it scans the current directory recursively.
- By default it renames every `.NEV` file to `.R3D`.
- `--invert` swaps the direction (`.R3D` → `.NEV`).
- If a destination filename already exists, the original file is left untouched and logged.
- Symlinks and unreadable paths are skipped with warnings so your media stays safe.

### Examples

```
# Convert everything under the current working directory
r3dy

# Target a specific card dump
r3dy /Volumes/CAM_DAY01

# Undo a conversion
r3dy --invert /Volumes/Archive/NRAW_backup
```

## Development

- `cargo run -- <path>` to try changes quickly.
- `cargo fmt && cargo clippy` before opening a PR.
- `cargo build --release` for a production binary.

## Notes

- The progress bar animates best on a real TTY. Log output keeps you informed even when piping or redirecting output.
- Renaming is instantaneous and lossless—no transcoding steps involved.
