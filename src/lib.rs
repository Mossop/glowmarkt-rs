use std::{collections::HashMap, fmt::Display};

use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use time::{Duration, OffsetDateTime, UtcOffset};

mod error;

pub use error::Error;

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    pub resource_id: String,
    pub resource_type_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VirtualEntity {
    #[serde(rename(deserialize = "veId"))]
    pub id: String,
    pub name: String,
    pub active: bool,
    #[serde(rename(deserialize = "veTypeId"))]
    pub type_id: String,
    pub owner_id: String,
    pub resources: Vec<ResourceInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProtocol {
    pub protocol: String,
    pub sensors: Vec<ResourceInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(rename(deserialize = "deviceId"))]
    pub id: String,
    pub description: String,
    pub active: bool,
    pub hardware_id: String,
    pub device_type_id: String,
    pub hardware_ids: HashMap<String, String>,
    pub protocol: DeviceProtocol,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename(deserialize = "resourceId"))]
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    #[serde(rename(deserialize = "resourceTypeId"))]
    pub type_id: String,
    pub owner_id: String,
    #[serde(rename(deserialize = "classifier"))]
    pub class: String,
    pub base_unit: String,
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

#[derive(Debug, Clone)]
pub struct GlowmarktEndpoint {
    pub base_url: String,
    pub app_id: String,
}

impl Default for GlowmarktEndpoint {
    fn default() -> Self {
        Self {
            base_url: BASE_URL.to_string(),
            app_id: APPLICATION_ID.to_string(),
        }
    }
}

impl GlowmarktEndpoint {
    pub async fn authenticate(
        self,
        username: String,
        password: String,
    ) -> Result<GlowmarktApi, Error> {
        let client = Client::new();
        let request = client
            .post(self.url("auth"))
            .json(&AuthRequest { username, password });

        let response: AuthResponse = self
            .api_call(&client, request)
            .await
            .map_err(|e| format!("Error authenticating: {}", e))?;

        if !response.valid {
            return Error::err("Authentication error");
        }

        log::debug!("Authenticated with API until {}", iso(response.expiry));

        Ok(GlowmarktApi {
            token: response.token,
            endpoint: self,
            client,
        })
    }

    fn url<S: Display>(&self, path: S) -> String {
        format!("{}/{}", self.base_url, path)
    }

    async fn api_call<T>(&self, client: &Client, request: RequestBuilder) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let request = request
            .header("applicationId", &self.app_id)
            .header("Content-Type", "application/json")
            .build()?;

        log::debug!("Sending {} request to {}", request.method(), request.url());
        let response = client.execute(request).await?;

        if !response.status().is_success() {
            log::error!("API returned error: {}", response.status());
            return Error::err(format!(
                "API returned unexpected response: {}",
                response.status()
            ));
        }

        let result = response.text().await?;
        log::trace!("Received: {}", result);

        Ok(serde_json::from_str::<T>(&result)?)
    }
}

struct ApiRequest<'a> {
    endpoint: &'a GlowmarktEndpoint,
    client: &'a Client,
    request: RequestBuilder,
}

impl<'a> ApiRequest<'a> {
    async fn request<T: DeserializeOwned>(self) -> Result<T, Error> {
        self.endpoint.api_call(self.client, self.request).await
    }
}

#[derive(Debug, Clone)]
pub struct GlowmarktApi {
    pub token: String,
    endpoint: GlowmarktEndpoint,
    client: Client,
}

impl GlowmarktApi {
    pub async fn authenticate(username: String, password: String) -> Result<GlowmarktApi, Error> {
        GlowmarktEndpoint::default()
            .authenticate(username, password)
            .await
    }

    fn get_request<S>(&self, path: S) -> ApiRequest
    where
        S: Display,
    {
        let request = self
            .client
            .get(self.endpoint.url(path))
            .header("token", &self.token);

        ApiRequest {
            endpoint: &self.endpoint,
            client: &self.client,
            request,
        }
    }

    fn query_request<S, T>(&self, path: S, query: &T) -> ApiRequest
    where
        S: Display,
        T: Serialize + ?Sized,
    {
        let request = self
            .client
            .get(self.endpoint.url(path))
            .header("token", &self.token)
            .query(query);

        ApiRequest {
            endpoint: &self.endpoint,
            client: &self.client,
            request,
        }
    }

    // fn post_request<S, T>(&self, path: S, data: &T) -> ApiRequest
    // where
    //     S: Display,
    //     T: Serialize,
    // {
    //     let request = self
    //         .client
    //         .post(self.endpoint.url(path))
    //         .header("Content-Type", "application/json")
    //         .header("token", &self.token)
    //         .json(data);

    //     ApiRequest {
    //         endpoint: &self.endpoint,
    //         client: &self.client,
    //         request,
    //     }
    // }

    pub async fn devices(&self) -> Result<Vec<Device>, Error> {
        self.get_request("device")
            .request()
            .await
            .map_err(|e| Error::from(format!("Error accessing devices: {}", e)))
    }

    pub async fn virtual_entities(&self) -> Result<Vec<VirtualEntity>, Error> {
        self.get_request("virtualentity")
            .request()
            .await
            .map_err(|e| Error::from(format!("Error accessing virtual entities: {}", e)))
    }

    pub async fn virtual_entity(&self, entity_id: &str) -> Result<VirtualEntity, Error> {
        self.get_request(format!("virtualentity/{}", entity_id))
            .request()
            .await
            .map_err(|e| Error::from(format!("Error accessing virtual entities: {}", e)))
    }

    pub async fn resource(&self, resource_id: &str) -> Result<Resource, Error> {
        self.get_request(format!("resource/{}", resource_id))
            .request()
            .await
            .map_err(|e| Error::from(format!("Error accessing virtual entities: {}", e)))
    }

    pub async fn readings(
        &self,
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

        let readings = self
            .query_request(
                format!("resource/{}/readings", resource_id),
                &[
                    ("from", iso(start.to_offset(UtcOffset::UTC))),
                    ("to", iso(end.to_offset(UtcOffset::UTC))),
                    ("period", period_arg),
                    ("offset", 0.to_string()),
                    ("function", "sum".to_string()),
                ],
            )
            .request::<ReadingsResponse>()
            .await
            .map_err(|e| Error::from(format!("Error accessing resource readings: {}", e)))?;

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
