use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha224};
use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::fs::File;
use std::iter;
use std::path::Path;
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
    Install {
        #[clap(long)]
        clean: bool,
    },
    Run {
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Generate {
        #[clap(long)]
        pyright: bool,
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

    let venvs_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::format_err!("data directory not found"))?
        .join(env!("CARGO_BIN_NAME"))
        .join("venvs");
    let venv_name = BASE64_URL_SAFE_NO_PAD.encode(Sha224::digest(
        working_dir
            .join(CONFIG_BASENAME)
            .to_string_lossy()
            .as_bytes(),
    ));
    let venv_dir = venvs_dir.join(&venv_name);

    match opts.command {
        Command::Install { clean } => {
            let config =
                toml::from_str::<Config>(&fs::read_to_string(working_dir.join(CONFIG_BASENAME))?)?;
            tracing::debug!("config = {:#?}", config);

            exec(
                process::Command::new("pyenv")
                    .arg("install")
                    .arg("--skip-existing")
                    .arg(&config.python),
            )?;
            if clean {
                fs::remove_dir_all(&venv_dir)?;
            }
            exec(
                process::Command::new("pyenv")
                    .arg("exec")
                    .arg("python")
                    .arg("-m")
                    .arg("venv")
                    .arg(&venv_dir)
                    .env("PYENV_VERSION", &config.python),
            )?;
            let mut command = process::Command::new(venv_dir.join("bin").join("pip"));
            command.arg("install");
            if let Some(index_url) = &config.index_url {
                command.arg("--index-url").arg(index_url);
            }
            for extra_index_url in &config.extra_index_urls {
                command.arg("--extra-index-url").arg(extra_index_url);
            }
            for (name, package) in &config.packages {
                match package {
                    Package::Index { version } => {
                        let mut requirement = name.to_owned();
                        if let Some(version) = version {
                            write!(&mut requirement, "{version}").unwrap();
                        }
                        command.arg(requirement);
                    }
                    Package::Local { path, editable } => {
                        if *editable {
                            command.arg("--editable");
                        }
                        if path.is_relative() {
                            command.arg(working_dir.join(path))
                        } else {
                            command.arg(path)
                        };
                    }
                }
            }
            exec(&mut command)?;
        }
        Command::Run { args } => {
            if !venv_dir.join("bin").exists() {
                anyhow::bail!("venv not installed");
            }
            exec(
                process::Command::new(&args[0]).args(&args[1..]).env(
                    "PATH",
                    env::join_paths(
                        [venv_dir.join("bin")]
                            .into_iter()
                            .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
                    )?,
                ),
            )?;
        }
        Command::Generate { pyright } => {
            if pyright {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Config<'a> {
                    venv_path: &'a Path,
                    venv: &'a str,
                }

                serde_json::to_writer(
                    File::create(working_dir.join("pyrightconfig.json"))?,
                    &Config {
                        venv_path: &venvs_dir,
                        venv: &venv_name,
                    },
                )?;
                tracing::info!("{:?}", working_dir.join("pyrightconfig.json"));
            }
        }
    }
    Ok(())
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize)]
struct Config {
    python: String,
    index_url: Option<String>,
    #[serde(default)]
    extra_index_urls: Vec<String>,
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
