use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha224};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::iter;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    Install,
    Run {
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    const CONFIG_BASENAME: &str = concat!(env!("CARGO_BIN_NAME"), ".toml");

    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    let current_dir = env::current_dir();
    let working_dir = iter::successors(current_dir.as_deref().ok(), |path| path.parent())
        .find(|dir| dir.join(CONFIG_BASENAME).exists())
        .ok_or_else(|| anyhow::format_err!("{} not found", CONFIG_BASENAME))?;

    let name = BASE64_URL_SAFE_NO_PAD.encode(Sha224::digest(
        working_dir
            .join(CONFIG_BASENAME)
            .to_string_lossy()
            .as_bytes(),
    ));
    let venv = dirs::data_dir()
        .ok_or_else(|| anyhow::format_err!("data directory not found"))?
        .join(env!("CARGO_BIN_NAME"))
        .join("venvs")
        .join(name);

    match opts.command {
        Command::Install => {
            let config =
                toml::from_str::<Config>(&fs::read_to_string(working_dir.join(CONFIG_BASENAME))?)?;
            tracing::debug!("config = {:#?}", config);

            exec(
                process::Command::new("pyenv")
                    .arg("install")
                    .arg("--skip-existing")
                    .arg(&config.python),
            )?;
            exec(
                process::Command::new("pyenv")
                    .arg("exec")
                    .arg("python")
                    .arg("-m")
                    .arg("venv")
                    .arg(&venv)
                    .env("PYENV_VERSION", &config.python),
            )?;
            exec(config.packages.iter().fold(
                process::Command::new(venv.join("bin").join("pip")).arg("install"),
                |command, (name, package)| match package {
                    Package::Index { version } => {
                        let mut requirement = name.to_owned();
                        if let Some(version) = version {
                            write!(&mut requirement, "{version}").unwrap();
                        }
                        command.arg(requirement)
                    }
                    Package::Local { path, editable } => {
                        let path = if path.is_relative() {
                            Cow::Owned(working_dir.join(path))
                        } else {
                            Cow::Borrowed(path)
                        };
                        if *editable {
                            command.arg("--editable").arg(path.as_ref())
                        } else {
                            command.arg(path.as_ref())
                        }
                    }
                },
            ))?;
        }
        Command::Run { args } => {
            if !venv.join("bin").exists() {
                anyhow::bail!("venv not installed");
            }
            exec(
                process::Command::new(&args[0]).args(&args[1..]).env(
                    "PATH",
                    env::join_paths(
                        [venv.join("bin")]
                            .into_iter()
                            .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
                    )?,
                ),
            )?;
        }
    }
    Ok(())
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize)]
struct Config {
    python: String,
    #[serde(default)]
    #[serde_as(as = "HashMap<_, serde_with::PickFirst<(_, serde_with::DisplayFromStr)>>")]
    packages: HashMap<String, Package>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum Package {
    Index {
        version: Option<pep440_rs::VersionSpecifiers>,
    },
    Local {
        path: PathBuf,
        #[serde(default)]
        editable: bool,
    },
}

impl FromStr for Package {
    type Err = <pep440_rs::VersionSpecifiers as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Index {
            version: Some(s.parse()?),
        })
    }
}

#[tracing::instrument]
fn exec(command: &mut process::Command) -> anyhow::Result<()> {
    tracing::debug!("exec");
    let status = command.status()?;
    tracing::debug!("status = {:?}", status);
    if !status.success() {
        anyhow::bail!("command failed");
    }
    Ok(())
}
