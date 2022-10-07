use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use secrecy::Secret;
use url::Url;

mod format;
mod sync;

#[derive(Parser, Clone, Debug)]
struct Synchronizer {
    #[clap(subcommand)]
    command: Command,
}

impl Synchronizer {
    async fn run() -> Result<()> {
        let synchronizer = Synchronizer::parse();

        synchronizer.run_command().await
    }

    async fn run_command(self) -> Result<()> {
        match self.command {
            Command::Sync {
                data_path,
                images_path,
                engine_url,
                engine_key,
                sync_images_bucket,
            } => {
                sync::sync(
                    data_path,
                    images_path,
                    engine_url,
                    engine_key,
                    sync_images_bucket,
                )
                .await?;
            }
            Command::Format {
                data_path,
                images_path,
            } => {
                format::format(data_path, images_path).await?;
            }
        }

        Ok(())
    }
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    Format {
        #[clap(
            short,
            long,
            value_parser,
            value_name = "PATH",
            default_value = "./data"
        )]
        data_path: PathBuf,

        #[clap(
            short,
            long,
            value_parser,
            value_name = "PATH",
            default_value = "./images"
        )]
        images_path: PathBuf,
    },
    Sync {
        #[clap(
            short,
            long,
            value_parser,
            value_name = "PATH",
            default_value = "./data"
        )]
        data_path: PathBuf,

        #[clap(
            short,
            long,
            value_parser,
            value_name = "PATH",
            default_value = "./images"
        )]
        images_path: PathBuf,

        #[clap(long)]
        sync_images_bucket: bool,

        #[clap(long, value_parser, value_name = "ENGINE_URL", env = "ENGINE_URL")]
        engine_url: Url,

        #[clap(long, value_parser, value_name = "ENGINE_KEY", env = "ENGINE_KEY")]
        engine_key: Secret<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "env-file")]
    {
        dotenvy::dotenv().ok();
    }

    Synchronizer::run().await
}
