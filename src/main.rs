use std::fmt::Display;

use clap::{Parser, Subcommand, ValueEnum};
use flexi_logger::Logger;
use glowmarkt::{Glowmarkt, ReadingPeriod};
use serde_json::to_string_pretty;
use time::{format_description::well_known::Iso8601, OffsetDateTime};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// A plain text format for humans to read.
    Text,
    /// JSON for processing by other tools.
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Period {
    HalfHour,
    Hour,
    Day,
    Week,
}

impl From<Period> for ReadingPeriod {
    fn from(period: Period) -> Self {
        match period {
            Period::HalfHour => ReadingPeriod::HalfHour,
            Period::Hour => ReadingPeriod::Hour,
            Period::Day => ReadingPeriod::Day,
            Period::Week => ReadingPeriod::Week,
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, env)]
    pub username: Option<String>,
    #[clap(short, long, env)]
    pub password: Option<String>,
    #[clap(short, long, env, arg_enum, value_parser, default_value_t = Format::Text)]
    pub format: Format,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lists meters
    List,
    /// Lists meter readings
    Readings {
        /// The meter to read.
        resource: String,
        /// Start time of first reading.
        from: String,
        /// Start time of last reading (defaults to now).
        to: Option<String>,

        /// The period of readings.
        #[clap(short, long, arg_enum, value_parser, default_value_t = Period::Hour)]
        period: Period,
    },
}

trait ErrorStr<V> {
    fn str_err(self) -> Result<V, String>;
}

impl<V, D: Display> ErrorStr<V> for Result<V, D> {
    fn str_err(self) -> Result<V, String> {
        self.map_err(|e| format!("Error: {}", e))
    }
}

async fn list(api: Glowmarkt, format: Format) -> Result<(), String> {
    let entities = api.virtual_entities().await.str_err()?;

    match format {
        Format::Json => println!("{}", to_string_pretty(&entities).str_err()?),
        Format::Text => {
            for entity in entities {
                for resource in entity.resources {
                    println!("{} {} ({})", resource.id, resource.name, resource.base_unit);
                }
            }
        }
    }

    Ok(())
}

async fn readings(
    api: Glowmarkt,
    format: Format,
    resource: String,
    start: String,
    end: Option<String>,
    period: Period,
) -> Result<(), String> {
    let start = OffsetDateTime::parse(&start, &Iso8601::DEFAULT).str_err()?;
    let end = if let Some(end) = end {
        OffsetDateTime::parse(&end, &Iso8601::DEFAULT).str_err()?
    } else {
        OffsetDateTime::now_utc()
    };

    let readings = api
        .readings(&resource, start, end, period.into())
        .await
        .str_err()?;

    match format {
        Format::Json => println!("{}", to_string_pretty(&readings).str_err()?),
        Format::Text => {}
    }

    Ok(())
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
        Command::List => list(api, args.format).await,
        Command::Readings {
            resource,
            from,
            to,
            period,
        } => readings(api, args.format, resource, from, to, period).await,
    }
}
