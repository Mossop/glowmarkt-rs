use std::{collections::HashMap, fmt::Display};

use clap::{Parser, Subcommand};
use flexi_logger::Logger;
use glowmarkt::{Device, Error, GlowmarktApi, ReadingPeriod, Resource};
use influx::Measurement;
use serde_json::to_string_pretty;
use time::{format_description::well_known::Iso8601, OffsetDateTime};

use crate::influx::{tags_for_device, tags_for_resource};

mod influx;

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
    /// Lists devices.
    Devices,
    /// Lists device types.
    DeviceTypes,
    /// Lists resource types.
    ResourceTypes,
    /// Displays all known resources.
    Resources,
    /// Displays details for a resource.
    Resource {
        /// The resource to display.
        resource_id: String,
    },
    /// Lists meter readings.
    Readings {
        /// The resource to read.
        resource_id: String,
        /// Start time of first reading.
        from: String,
        /// Start time of last reading (defaults to now).
        to: Option<String>,
    },
    /// Retrieves device data in InfluxDB line protocol.
    Influx {
        /// The device to read. If absent all devices are read.
        #[clap(short, long, env)]
        device: Option<String>,
        /// Start time of first reading.
        from: String,
        /// Start time of last reading (defaults to now).
        to: Option<String>,
    },
}

fn parse_date(date: String) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(&date, &Iso8601::DEFAULT).str_err()
}

fn parse_end_date(date: Option<String>) -> Result<OffsetDateTime, String> {
    if let Some(date) = date {
        OffsetDateTime::parse(&date, &Iso8601::DEFAULT).str_err()
    } else {
        Ok(OffsetDateTime::now_utc())
    }
}

trait ErrorStr<V> {
    fn str_err(self) -> Result<V, String>;
}

impl<V, D: Display> ErrorStr<V> for Result<V, D> {
    fn str_err(self) -> Result<V, String> {
        self.map_err(|e| format!("Error: {}", e))
    }
}

fn values<T>(map: HashMap<String, T>) -> Vec<T> {
    map.into_values().collect()
}

async fn devices(api: GlowmarktApi) -> Result<(), String> {
    let devices = api.devices().await.str_err()?;
    println!("{}", to_string_pretty(&values(devices)).str_err()?);
    Ok(())
}

async fn device_types(api: GlowmarktApi) -> Result<(), String> {
    let types = api.device_types().await.str_err()?;
    println!("{}", to_string_pretty(&values(types)).str_err()?);
    Ok(())
}

async fn resources(api: GlowmarktApi) -> Result<(), String> {
    let resources = api.resources().await.str_err()?;
    println!("{}", to_string_pretty(&values(resources)).str_err()?);
    Ok(())
}

async fn resource_types(api: GlowmarktApi) -> Result<(), String> {
    let types = api.resource_types().await.str_err()?;
    println!("{}", to_string_pretty(&values(types)).str_err()?);
    Ok(())
}

async fn readings(
    api: GlowmarktApi,
    resource: String,
    start: String,
    end: Option<String>,
) -> Result<(), String> {
    let start = parse_date(start)?;
    let end = parse_end_date(end)?;

    let readings = api
        .readings(&resource, &start, &end, ReadingPeriod::HalfHour)
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

async fn influx(
    api: GlowmarktApi,
    device: Option<String>,
    start: String,
    end: Option<String>,
) -> Result<(), String> {
    let start = parse_date(start)?;
    let end = parse_end_date(end)?;
    let mut measurements = Vec::new();

    let resources = api.resources().await?;

    async fn process_device(
        api: &GlowmarktApi,
        resources: &HashMap<String, Resource>,
        device: Device,
        start: &OffsetDateTime,
        end: &OffsetDateTime,
        measurements: &mut Vec<Measurement>,
    ) -> Result<(), Error> {
        let tags = tags_for_device(&device);

        for sensor in device.protocol.sensors {
            if let Some(resource) = resources.get(&sensor.resource_id) {
                let tags = tags_for_resource(&tags, resource);
                let readings = api
                    .readings(&resource.id, start, end, ReadingPeriod::HalfHour)
                    .await?;

                for reading in readings {
                    let mut measurement =
                        Measurement::new("glowmarkt", reading.start, tags.clone());
                    measurement.add_field("value", reading.value as f64);
                    measurements.push(measurement);
                }
            }
        }

        Ok(())
    }

    if let Some(device) = device {
        if let Some(device) = api.device(&device).await? {
            process_device(&api, &resources, device, &start, &end, &mut measurements).await?;
        } else {
            eprintln!("Error: Unknown device {}", device);
        }
    } else {
        let devices = api.devices().await?.into_values();
        for device in devices {
            process_device(&api, &resources, device, &start, &end, &mut measurements).await?;
        }
    }

    measurements.sort_unstable_by(|m1, m2| m1.timestamp.partial_cmp(&m2.timestamp).unwrap());
    for measurement in measurements {
        println!("{}", measurement);
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
        GlowmarktApi::authenticate(username, password).await?
    } else {
        return Err("Must pass username and password.".to_string());
    };

    match args.command {
        Command::Devices => devices(api).await,
        Command::DeviceTypes => device_types(api).await,
        Command::ResourceTypes => resource_types(api).await,
        Command::Resources => resources(api).await,
        Command::Resource { resource_id } => resource(api, resource_id).await,
        Command::Readings {
            resource_id,
            from,
            to,
        } => readings(api, resource_id, from, to).await,
        Command::Influx { device, from, to } => influx(api, device, from, to).await,
    }
}
