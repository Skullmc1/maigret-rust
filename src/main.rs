mod checker;
mod models;
mod web;

use crate::models::MaigretCheckStatus;
use anyhow::{Context, Result};
use checker::Maigret;
use clap::Parser;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Username to search for
    #[arg(short, long)]
    username: Option<String>,

    /// Path to the sites database (data.json)
    #[arg(
        short,
        long,
        default_value = "../maigret-main/maigret/resources/data.json"
    )]
    db: String,

    /// Number of concurrent checks
    #[arg(short, long, default_value_t = 50)]
    concurrency: usize,

    /// Start web server
    #[arg(short, long)]
    web: bool,

    /// Web server port
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let db_path = Path::new(&args.db);
    let mut file = File::open(db_path)
        .with_context(|| format!("Failed to open database file: {:?}", db_path))?;
    let mut json_content = String::new();
    file.read_to_string(&mut json_content)?;

    let db: models::MaigretDatabase =
        serde_json::from_str(&json_content).context("Failed to parse database JSON")?;

    println!("Loaded {} sites from database.", db.sites.len());

    if args.web {
        web::start_server(db, args.port).await;
    } else if let Some(username) = args.username {
        println!("Searching for username: {}", username);
        let maigret = Maigret::new(db);

        let results = maigret.search(vec![username], args.concurrency).await;

        println!("\nFinal Summary:");
        println!("----------------------------------------");
        let mut found_count = 0;
        for result in results {
            if result.status == MaigretCheckStatus::Claimed {
                println!("[+] {}: Found!", result.site_name);
                found_count += 1;

                if !result.ids_data.is_empty() {
                    println!("    Extracted IDs:");
                    for (id, kind) in &result.ids_data {
                        println!("      - {}: {}", kind, id);
                    }
                }
            }
        }
        println!("----------------------------------------");
        println!("Found {} accounts in total.", found_count);
    } else {
        println!("Please provide a username to search or start the web server with --web.");
    }

    Ok(())
}
