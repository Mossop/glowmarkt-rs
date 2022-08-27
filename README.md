# Glowmarkt

[![Crates.io](https://img.shields.io/crates/v/glowmarkt)](https://crates.io/crates/glowmarkt)
[![docs.rs](https://img.shields.io/docsrs/glowmarkt)](https://docs.rs/glowmarkt)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)

A rust crate for accessing the Glowmarkt API for meter readings.

This contains two parts. A module that other programs can use for programmatic
access to the API and a CLI that users can use to query for data. It was
developed with the primary purpose of being able to submit meter readings to
[InfluxDB](https://www.influxdata.com/products/influxdb-overview/) but along the
way a number of other ways of displaying data from the API were added to the CLI.

In order to use this you need to have installed and registered your smart meters
with the [Bright mobile app](https://play.google.com/store/apps/details?id=uk.co.hildebrand.brightionic).
Then you can use the username and password you used there with this tool.

## CLI Usage

The CLI should be reasonably well documented with `--help`.

```shell
$> cargo install glowmarkt
$> glowmarkt --username='me@somewhere.com' --password='wibble' device
```

## Module Usage

The API is async so you must set up an async runtime such as tokio.
Authenticating with a username and password will generate a token for subsequent
requests.

```rust
let api = GlowmarktApi::authenticate("me@somewhere.com", "wibble").await?;
let devices = api.devices().await?;
```

Consult the [module docs](https://docs.rs/glowmarkt) for more information.
