use std::{collections::HashSet, error::Error};

use async_std::fs::read_dir;
use clap::Parser;
use db::IndexDB;

mod config_reader;
mod db;
mod format;
mod helper;
mod indexer;
mod parser;
mod schema;
mod search;
use error::FOError;
use indexer::Indexer;

use crate::{helper::FileHelper, mover::Mover, schema::SchemaList};
mod error;
mod mover;

#[derive(Parser, Debug)]
struct CliArgs {
    #[command(subcommand)]
    command: Subcommand,
    #[arg(short, long, default_value = "./")]
    path: String,
}

// #[derive(Clone, Parser, clap::ValueEnum)]
// enum CliMode {
//     Search,
//     DebugMove,
// }

#[derive(clap::Subcommand, Debug, Clone)]
enum Subcommand {
    Search { search: Vec<String> },
    DebugMove,
}

// #[derive(Debug, Clone)]
// enum CliMode {
//     DebugMove,
// }

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();
    let mut recommendation = HashSet::new();
    // dbg!(args);
    match &args.command {
        Subcommand::Search { search } => {
            println!("to be implemented")
        }
        Subcommand::DebugMove => {
            //TODO: to fucking do. support for using field before in tree before it used
            let helper = FileHelper::new(&args.path);
            let config = helper.read_config()?;
            let sl = SchemaList::from(&config.schema);
            for file in helper.read_dir()? {
                let mover = Mover::new(file.get_path());
                let path_to = mover.get_path(&config, &sl);
                if let Err(FOError::PatternError(_)) = path_to {
                    recommendation.insert("don't forgot to add _import and make sure it's valid");
                }
                println!(
                    "{:?} -> {}",
                    file.get_path(),
                    path_to.unwrap_or("ignored".to_owned())
                )
            }

            // let config = serde_yaml::from_reader(rdr)
        }
    }
    println!("tips:");
    for tip in recommendation {
        println!(" - {}", tip);
    }
    Ok(())
}
