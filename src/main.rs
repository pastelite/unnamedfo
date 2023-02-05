use std::error::Error;

use clap::Parser;
use db::IndexDB;

mod db;
mod indexer;
mod path;
mod search;

use indexer::Indexer;

#[derive(Parser, Debug)]
struct CliArgs {
    search: Vec<String>,
    #[arg(short, long, default_value = "./")]
    path: String,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = CliArgs::parse();

    if cfg!(debug_assertions) {
        args.path = "./testdir".to_string();
    }

    println!("{:#?}", args);

    let mut db = IndexDB::open(&args.path).await?;

    Indexer::open(&mut db).indexing("./", 0).await?;

    // indexer("./", &mut db, 0).await?;
    Ok(())
}
