use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

pub use clap;
pub use xshell;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli<T: Subcommand> {
    #[command(subcommand)]
    pub command: Commands<T>,
}

#[derive(Subcommand)]
pub enum Commands<T: Subcommand> {
    #[command(flatten)]
    Common(CommonCommands),
    #[command(flatten)]
    Custom(T),
}

#[derive(Subcommand)]
pub enum Empty {}

impl Commands<Empty> {
    pub fn execute(&self, name: &str) -> Result<()> {
        match &self {
            Self::Common(common_commands) => common_commands.execute(name),
            Self::Custom(_) => Ok(()),
        }
    }
}

#[derive(Subcommand)]
pub enum CommonCommands {
    /// Build the project
    Build,
    /// Install the project
    Install {
        #[arg(long, default_value = "/")]
        destdir: PathBuf,
        #[arg(long, default_value = "usr")]
        prefix: PathBuf,
        #[arg(long, default_value = "755")]
        mode: String,
    },
}

impl CommonCommands {
    pub fn execute(&self, name: &str) -> Result<()> {
        match &self {
            Self::Build => build(),
            Self::Install {
                destdir,
                prefix,
                mode,
            } => install(name, destdir, prefix, mode),
        }
    }
}

const RUSTFLAGS: &[&str] = &[
    "-C link-arg=-fuse-ld=lld",
    "-C linker-plugin-lto",
    "-C link-arg=-Wl,-z,pack-relative-relocs",
    "-C link-arg=-Wl,-O1",
    "-C link-arg=-Wl,--gc-sections",
    "-C link-arg=-Wl,--as-needed",
    "-C link-arg=-Wl,-z,now",
    "-C link-arg=-Wl,-z,relro",
];

fn build() -> Result<()> {
    let sh = Shell::new()?;
    println!("Building release version...");
    cmd!(sh, "cargo build --release --locked")
        .env("RUSTFLAGS", RUSTFLAGS.join(" "))
        .run()?;
    Ok(())
}

fn install(name: &str, destdir: &Path, prefix: &Path, mode: &str) -> Result<()> {
    let target_dir = format!("target/release/{name}");
    if !fs::exists(&target_dir)? {
        bail!("You must build the project first!")
    }

    let binary_dir = destdir.join(prefix).join("bin");

    // Create target directory if it doesn't exist
    fs::create_dir_all(&binary_dir).context("Failed to create binary directory")?;

    let target = binary_dir.join(name);

    fs::copy(target_dir, &target)
        .with_context(|| format!("Failed to copy binary to {:?}", target))?;

    // Parse octal mode string (e.g., "755" or "0755")
    let mode = u32::from_str_radix(mode.trim_start_matches('0'), 8)
        .with_context(|| format!("Invalid mode: {mode}"))?;

    fs::set_permissions(&target, fs::Permissions::from_mode(mode))
        .context("Failed to set binary permissions")?;

    println!("Installation complete!");
    Ok(())
}
