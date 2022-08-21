use std::fmt::Display;

use clap::{Parser, Subcommand};
use flexi_logger::Logger;
use glowmarkt::{GlowmarktApi, ReadingPeriod};
use serde_json::to_string_pretty;
use time::{format_description::well_known::Iso8601, OffsetDateTime};

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
    /// Lists devices
    Devices,
    /// Lists device types
    DeviceTypes,
    /// Lists resource types
    ResourceTypes,
    /// Displays details for a resource
    Resource {
        /// The resource to display.
        resource_id: String,
    },
    /// Lists meter readings
    Readings {
        /// The resource to read.
        resource_id: String,
        /// Start time of first reading.
        from: String,
        /// Start time of last reading (defaults to now).
        to: Option<String>,
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

async fn devices(api: GlowmarktApi) -> Result<(), String> {
    let devices = api.devices().await.str_err()?;
    println!("{}", to_string_pretty(&devices).str_err()?);
    Ok(())
}

async fn device_types(api: GlowmarktApi) -> Result<(), String> {
    let types = api.device_types().await.str_err()?;
    println!("{}", to_string_pretty(&types).str_err()?);
    Ok(())
}

async fn resource_types(api: GlowmarktApi) -> Result<(), String> {
    let types = api.resource_types().await.str_err()?;
    println!("{}", to_string_pretty(&types).str_err()?);
    Ok(())
}

async fn readings(
    api: GlowmarktApi,
    resource: String,
    start: String,
    end: Option<String>,
) -> Result<(), String> {
    let start = OffsetDateTime::parse(&start, &Iso8601::DEFAULT).str_err()?;
    let end = if let Some(end) = end {
        OffsetDateTime::parse(&end, &Iso8601::DEFAULT).str_err()?
    } else {
        OffsetDateTime::now_utc()
    };

    let readings = api
        .readings(&resource, start, end, ReadingPeriod::HalfHour)
        .await
        .str_err()?;

    println!("{}", to_string_pretty(&readings).str_err()?);
    Ok(())
}

async fn resource(api: GlowmarktApi, resource: String) -> Result<(), String> {
    let readings = api.resource(&resource).await.str_err()?;

    println!("{}", to_string_pretty(&readings).str_err()?);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    if let Err(e) = Logger::try_with_env_or_str("info").and_then(|logger| logger.start()) {
        eprintln!("Warning, failed to start logging: {}", e);
    }

    let args = Args::parse();

    let api = if let (Some(username), Some(password)) = (args.username, args.password) {
        GlowmarktApi::authenticate(username, password).await?
    } else {
        return Err("Must pass username and password.".to_string());
    };

    match args.command {
        Command::Devices => devices(api).await,
        Command::DeviceTypes => device_types(api).await,
        Command::ResourceTypes => resource_types(api).await,
        Command::Resource { resource_id } => resource(api, resource_id).await,
        Command::Readings {
            resource_id,
            from,
            to,
        } => readings(api, resource_id, from, to).await,
    }
}
