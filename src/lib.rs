//! Access to the Glowmarkt API for meter readings.
//!
//! Developed based on <https://bitbucket.org/ijosh/brightglowmarkt/src/master/>
#![warn(missing_docs)]

use std::{collections::HashMap, fmt::Display};

use error::maybe;
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};
use time::{OffsetDateTime, UtcOffset};

pub mod api;
pub mod error;

pub use api::{Device, DeviceType, Resource, ResourceType, VirtualEntity};
pub use error::{Error, ErrorKind};

/// The default API endpoint.
pub const BASE_URL: &str = "https://api.glowmarkt.com/api/v0-1";
/// The default application ID to use when communicating with the API.
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
/// The time window for each reading.
pub enum ReadingPeriod {
    /// 30 minutes.
    HalfHour,
    /// 1 hour.
    Hour,
    /// 1 day.
    Day,
    /// 1 week.
    Week,
    /// 1 month.
    Month,
    /// 1 year.
    Year,
}

trait Identified {
    fn id(&self) -> &str;
}

fn build_map<I: Identified>(list: Vec<I>) -> HashMap<String, I> {
    list.into_iter()
        .map(|v| (v.id().to_owned(), v))
        .collect::<HashMap<String, I>>()
}

impl Identified for api::VirtualEntity {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Identified for api::DeviceType {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Identified for api::Device {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Identified for api::ResourceType {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Identified for api::Resource {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Debug)]
/// A meter reading
pub struct Reading {
    #[serde(with = "time::serde::rfc3339")]
    /// The start time of the period.
    pub start: OffsetDateTime,
    /// The length of the period.
    #[serde(skip)]
    pub period: ReadingPeriod,
    /// The total usage.
    pub value: f32,
}

/// The API endpoint.
///
/// Normally a non-default endpoint would only be useful for testing purposes.
#[derive(Debug, Clone)]
pub struct GlowmarktEndpoint {
    /// The URL of the API endpoint.
    pub base_url: String,
    /// The application ID to use when communicating with the endpoint.
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
        let response = client
            .execute(request)
            .await?
            .error_for_status()
            .map_err(|e| {
                log::warn!("Received API error: {}", e);
                e
            })?;

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
/// Access to the Glowmarkt API.
pub struct GlowmarktApi {
    /// The current JWT token.
    pub token: String,
    endpoint: GlowmarktEndpoint,
    client: Client,
}

impl GlowmarktApi {
    /// Create with a provided JWT token.
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_owned(),
            endpoint: Default::default(),
            client: Client::new(),
        }
    }

    /// Authenticates with the default Glowmarkt API endpoint.
    ///
    /// Generates a valid JWT token if successful.
    pub async fn authenticate(username: &str, password: &str) -> Result<GlowmarktApi, Error> {
        Self::auth(Default::default(), username, password).await
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
}

/// [User System](https://api.glowmarkt.com/api-docs/v0-1/usersys/usertypes/)
impl GlowmarktApi {
    /// Authenticate against a specific endpoint.
    pub async fn auth(
        endpoint: GlowmarktEndpoint,
        username: &str,
        password: &str,
    ) -> Result<GlowmarktApi, Error> {
        let client = Client::new();
        let request = client.post(endpoint.url("auth")).json(&api::AuthRequest {
            username: username.to_owned(),
            password: password.to_owned(),
        });

        let response = endpoint
            .api_call::<api::AuthResponse>(&client, request)
            .await?
            .validate()?;

        log::debug!("Authenticated with API until {}", iso(response.expiry));

        Ok(Self {
            token: response.token,
            endpoint,
            client,
        })
    }

    /// Validates the current token.
    pub async fn validate(&self) -> Result<bool, Error> {
        let response = self
            .get_request("auth")
            .request::<api::ValidateResponse>()
            .await
            .and_then(|r| r.validate())?;

        log::debug!("Authenticated with API until {}", iso(response.expiry));

        Ok(true)
    }
}

/// [Device Management System](https://api.glowmarkt.com/api-docs/v0-1/dmssys/#/)
impl GlowmarktApi {
    /// Retrieves all of the known device types.
    pub async fn device_types(&self) -> Result<HashMap<String, api::DeviceType>, Error> {
        self.get_request("devicetype")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves all of the devices registered for an account.
    pub async fn devices(&self) -> Result<HashMap<String, api::Device>, Error> {
        self.get_request("device").request().await.map(build_map)
    }

    /// Retrieves a single device.
    pub async fn device(&self, id: &str) -> Result<Option<api::Device>, Error> {
        match self.get_request(format!("device/{}", id)).request().await {
            Ok(device) => Ok(Some(device)),
            Err(error) => {
                if error.kind == ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }
}

/// [Virtual Entity System](https://api.glowmarkt.com/api-docs/v0-1/vesys/#/)
impl GlowmarktApi {
    /// Retrieves all of the virtual entities registered for an account.
    pub async fn virtual_entities(&self) -> Result<HashMap<String, api::VirtualEntity>, Error> {
        self.get_request("virtualentity")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves a single virtual entity by ID.
    pub async fn virtual_entity(
        &self,
        entity_id: &str,
    ) -> Result<Option<api::VirtualEntity>, Error> {
        maybe(
            self.get_request(format!("virtualentity/{}", entity_id))
                .request()
                .await,
        )
    }
}

/// [Resource System](https://api.glowmarkt.com/api-docs/v0-1/resourcesys/#/)
impl GlowmarktApi {
    /// Retrieves all of the known resource types.
    pub async fn resource_types(&self) -> Result<HashMap<String, api::ResourceType>, Error> {
        self.get_request("resourcetype")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves all resources.
    pub async fn resources(&self) -> Result<HashMap<String, api::Resource>, Error> {
        self.get_request("resource").request().await.map(build_map)
    }

    /// Retrieves a single resource by ID.
    pub async fn resource(&self, resource_id: &str) -> Result<Option<api::Resource>, Error> {
        maybe(
            self.get_request(format!("resource/{}", resource_id))
                .request()
                .await,
        )
    }

    /// Retrieves the readings for a single resource.
    ///
    /// The API docs suggest that the start date should be set to the beginning
    /// of the week (Monday) when the period is `Week` and the beginning of the
    /// month when the period is `Month`. It is unclear what role the timezone
    /// plays in this.
    ///
    /// The Glowmarkt API behaves strangely in the presence of non-UTC
    /// timezones so `start` and `end` will first be converted to UTC and all
    /// returned readings will be in UTC.
    pub async fn readings(
        &self,
        resource_id: &str,
        start: &OffsetDateTime,
        end: &OffsetDateTime,
        period: ReadingPeriod,
    ) -> Result<Vec<Reading>, Error> {
        let period_arg = match period {
            ReadingPeriod::HalfHour => "PT30M".to_string(),
            ReadingPeriod::Hour => "PT1H".to_string(),
            ReadingPeriod::Day => "P1D".to_string(),
            ReadingPeriod::Week => "P1W".to_string(),
            ReadingPeriod::Month => "P1M".to_string(),
            ReadingPeriod::Year => "P1Y".to_string(),
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
            .request::<api::ReadingsResponse>()
            .await?;

        Ok(readings
            .data
            .into_iter()
            .map(|(timestamp, value)| Reading {
                start: OffsetDateTime::from_unix_timestamp(timestamp).unwrap(),
                period,
                value,
            })
            .collect())
    }
}
