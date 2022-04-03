use clap::Parser;
use glowmarkt::Glowmarkt;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long)]
    pub username: Option<String>,
    #[clap(short, long)]
    pub password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();

    let api = if let (Some(username), Some(password)) = (args.username, args.password) {
        Glowmarkt::authenticate(username, password).await?
    } else {
        return Err("Must pass username and password.".to_string());
    };

    println!("Connected: {:?}", api);

    Ok(())
}
