use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha224};
use std::env;
use std::fs;
use std::iter;
use std::process;

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

fn main() {
    const CONFIG_BASENAME: &str = concat!(env!("CARGO_BIN_NAME"), ".toml");

    let opts = Opts::parse();

    let current_dir = env::current_dir();
    let working_dir = iter::successors(current_dir.as_deref().ok(), |path| path.parent())
        .find(|dir| dir.join(CONFIG_BASENAME).exists())
        .unwrap();

    let name = BASE64_URL_SAFE_NO_PAD.encode(Sha224::digest(
        working_dir
            .join(CONFIG_BASENAME)
            .to_string_lossy()
            .as_bytes(),
    ));
    let venv = dirs::data_dir()
        .unwrap()
        .join(env!("CARGO_BIN_NAME"))
        .join("venvs")
        .join(name);

    match opts.command {
        Command::Install => {
            let config = toml::from_str::<Config>(
                &fs::read_to_string(working_dir.join(CONFIG_BASENAME)).unwrap(),
            )
            .unwrap();

            assert!(process::Command::new("pyenv")
                .arg("install")
                .arg("--skip-existing")
                .arg(&config.python)
                .status()
                .unwrap()
                .success());
            assert!(process::Command::new("pyenv")
                .arg("exec")
                .arg("python")
                .arg("-m")
                .arg("venv")
                .arg(&venv)
                .env("PYENV_VERSION", &config.python)
                .status()
                .unwrap()
                .success());
        }
        Command::Run { args } => {
            assert!(process::Command::new(&args[0])
                .args(&args[1..])
                .env(
                    "PATH",
                    env::join_paths(
                        [venv.join("bin")]
                            .into_iter()
                            .chain(env::split_paths(&env::var_os("PATH").unwrap()))
                    )
                    .unwrap()
                )
                .status()
                .unwrap()
                .success());
        }
    }
}

#[derive(Deserialize)]
struct Config {
    python: String,
}
