use std::fmt::Display;

use reqwest::{Client, Request, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use time::{Duration, OffsetDateTime, UtcOffset};

use crate::Error;

// Developed based on https://bitbucket.org/ijosh/brightglowmarkt/src/master/

pub const BASE_URL: &str = "https://api.glowmarkt.com/api/v0-1";
pub const APPLICATION_ID: &str = "b0f1b774-a586-4f72-9edd-27ead8aa7a8d";

fn iso(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

#[derive(Debug, Clone, Copy)]
pub enum ReadingPeriod {
    HalfHour,
    Hour,
    Day,
    Week,
    // Month,
    // Year,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(from = "String")]
pub enum ResourceClass {
    Unknown,
    ElectricityUsage,
    ElectricityCost,
    GasUsage,
    GasCost,
}

impl From<String> for ResourceClass {
    fn from(str: String) -> ResourceClass {
        match str.as_str() {
            "electricity.consumption" => ResourceClass::ElectricityUsage,
            "electricity.consumption.cost" => ResourceClass::ElectricityCost,
            "gas.consumption" => ResourceClass::GasUsage,
            "gas.consumption.cost" => ResourceClass::GasCost,
            _ => {
                log::warn!("Unknown resource classifier: {}", str);
                ResourceClass::Unknown
            }
        }
    }
}

#[derive(Serialize, Debug)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub valid: bool,
    pub account_id: String,
    pub token: String,
    #[serde(rename = "exp", with = "time::serde::timestamp")]
    pub expiry: OffsetDateTime,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VirtualEntityBase {
    #[serde(rename(deserialize = "veId"))]
    pub id: String,
    #[serde(rename(deserialize = "veTypeId"))]
    pub type_id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDetail {
    #[serde(rename(deserialize = "resourceId"))]
    pub id: String,
    #[serde(rename(deserialize = "resourceTypeId"))]
    pub type_id: String,
    pub name: String,
    #[serde(rename(deserialize = "classifier"))]
    pub class: ResourceClass,
    pub description: String,
    pub base_unit: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VirtualEntityDetail {
    #[serde(rename(deserialize = "veId"))]
    pub id: String,
    #[serde(rename(deserialize = "veTypeId"))]
    pub type_id: String,
    pub name: String,
    pub resources: Vec<ResourceDetail>,
}

type ReadingTuple = (i64, f32);

#[derive(Serialize, Debug)]
pub struct Reading {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
    pub value: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReadingsResponse {
    pub data: Vec<ReadingTuple>,
}

async fn api_call<T>(client: &Client, request: Request) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    log::trace!("Sending {} request to {}", request.method(), request.url());
    let response = client.execute(request).await?;

    if !response.status().is_success() {
        return Error::err(format!(
            "API returned unexpected response: {}",
            response.status()
        ));
    }

    response.json::<T>().await.map_err(|e| e.to_string().into())
}

#[derive(Debug, Clone)]
pub struct GlowmarktEndpoint {
    pub base_url: String,
    pub app_id: String,
    client: Client,
}

impl Default for GlowmarktEndpoint {
    fn default() -> Self {
        Self {
            base_url: BASE_URL.to_string(),
            app_id: APPLICATION_ID.to_string(),
            client: Client::new(),
        }
    }
}

impl GlowmarktEndpoint {
    pub async fn authenticate(
        self,
        username: String,
        password: String,
    ) -> Result<crate::Glowmarkt, Error> {
        let response: AuthResponse = api_call(
            &self.client,
            self.post_request("auth", &AuthRequest { username, password })
                .build()?,
        )
        .await
        .map_err(|e| format!("Error authenticating: {}", e))?;

        if !response.valid {
            return Error::err("Authentication error");
        }

        log::trace!("Authenticated with API until {}", iso(response.expiry));

        Ok(crate::Glowmarkt {
            endpoint: self,
            token: response.token,
        })
    }

    pub(crate) fn get_request<S: Display>(&self, path: S) -> RequestBuilder {
        self.client
            .get(format!("{}/{}", self.base_url, path))
            .header("applicationId", &self.app_id)
    }

    pub(crate) fn post_request<T: Serialize>(&self, path: &str, data: &T) -> RequestBuilder {
        self.client
            .post(format!("{}/{}", self.base_url, path))
            .header("applicationId", &self.app_id)
            .header("Content-Type", "application/json")
            .json(data)
    }

    pub(crate) async fn virtual_entities(
        &self,
        token: &str,
    ) -> Result<Vec<VirtualEntityBase>, Error> {
        api_call(
            &self.client,
            self.get_request("virtualentity")
                .header("token", token)
                .build()?,
        )
        .await
        .map_err(|e| Error::from(format!("Error accessing virtual entities: {}", e)))
    }

    pub(crate) async fn virtual_entity(
        &self,
        token: &str,
        entity_id: &str,
    ) -> Result<VirtualEntityDetail, Error> {
        api_call(
            &self.client,
            self.get_request(format!("virtualentity/{}/resources", entity_id))
                .header("token", token)
                .build()?,
        )
        .await
        .map_err(|e| Error::from(format!("Error accessing virtual entities: {}", e)))
    }

    pub(crate) async fn readings(
        &self,
        token: &str,
        resource_id: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        period: ReadingPeriod,
    ) -> Result<Vec<Reading>, Error> {
        let period_arg = match period {
            ReadingPeriod::HalfHour => "PT30M".to_string(),
            ReadingPeriod::Hour => "PT1H".to_string(),
            ReadingPeriod::Day => "P1D".to_string(),
            ReadingPeriod::Week => "P1W".to_string(),
            // ReadingPeriod::Month => "P1M".to_string(),
            // ReadingPeriod::Year => "P1Y".to_string(),
        };

        let readings = api_call::<ReadingsResponse>(
            &self.client,
            self.get_request(format!("resource/{}/readings", resource_id))
                .query(&[
                    ("from", iso(start.to_offset(UtcOffset::UTC))),
                    ("to", iso(end.to_offset(UtcOffset::UTC))),
                    ("period", period_arg),
                    ("offset", 0.to_string()),
                    ("function", "sum".to_string()),
                ])
                .header("token", token)
                .build()?,
        )
        .await
        .map_err(|e| Error::from(format!("Error accessing resource readings: {}", e)))?;

        log::trace!("Received readings {:?}", readings.data);

        Ok(readings
            .data
            .into_iter()
            .map(|(timestamp, value)| {
                let start = OffsetDateTime::from_unix_timestamp(timestamp).unwrap();

                let end = match period {
                    ReadingPeriod::HalfHour => start + Duration::minutes(30),
                    ReadingPeriod::Hour => start + Duration::hours(1),
                    ReadingPeriod::Day => start + Duration::days(1),
                    ReadingPeriod::Week => start + Duration::weeks(1),
                };

                Reading { start, end, value }
            })
            .collect())
    }
}
