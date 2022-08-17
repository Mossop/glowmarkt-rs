use clap::{Parser, Subcommand};
use flexi_logger::Logger;
use glowmarkt::Glowmarkt;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, env)]
    pub username: Option<String>,
    #[clap(short, long, env)]
    pub password: Option<String>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lists meters
    List,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    if let Err(e) = Logger::try_with_env_or_str("info").and_then(|logger| logger.start()) {
        eprintln!("Warning, failed to start logging: {}", e);
    }

    let args = Args::parse();

    let api = if let (Some(username), Some(password)) = (args.username, args.password) {
        Glowmarkt::authenticate(username, password).await?
    } else {
        return Err("Must pass username and password.".to_string());
    };

    match args.command {
        Command::List => {
            let entities = api.virtual_entities().await?;
            for entity in entities {
                for resource in entity.resources {
                    println!("{} {} ({})", resource.id, resource.name, resource.base_unit);
                }
            }
        }
    }

    Ok(())
}
