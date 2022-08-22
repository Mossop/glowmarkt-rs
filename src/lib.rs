use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use error::maybe;
use reqwest::{Client, RequestBuilder};
use serde::{
    de::{self, DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use time::{OffsetDateTime, UtcOffset};

pub mod error;

pub use error::{Error, ErrorKind};

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

trait Identified {
    fn id(&self) -> &str;
}

fn build_map<I: Identified>(list: Vec<I>) -> HashMap<String, I> {
    list.into_iter()
        .map(|v| (v.id().to_owned(), v))
        .collect::<HashMap<String, I>>()
}

#[derive(Debug, Clone, Copy)]
pub enum ReadingPeriod {
    HalfHour,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Serialize, Debug)]
struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    pub message: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InvalidAuthResponse {
    pub error: ErrorResponse,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ValidAuthResponse {
    pub valid: bool,
    pub token: String,
    #[serde(rename = "exp", with = "time::serde::timestamp")]
    pub expiry: OffsetDateTime,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AuthResponse {
    Invalid(InvalidAuthResponse),
    Valid(ValidAuthResponse),
}

impl AuthResponse {
    pub fn validate(self) -> Result<ValidAuthResponse, Error> {
        match self {
            AuthResponse::Valid(response) => {
                if response.valid {
                    Ok(response)
                } else {
                    Err(Error {
                        kind: ErrorKind::NotAuthenticated,
                        message: "Authentication error".to_string(),
                    })
                }
            }
            AuthResponse::Invalid(response) => Err(Error {
                kind: ErrorKind::NotAuthenticated,
                message: response.error.message,
            }),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InvalidValidateResponse {
    pub error: ErrorResponse,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ValidValidateResponse {
    pub valid: bool,
    #[serde(rename = "exp", with = "time::serde::timestamp")]
    pub expiry: OffsetDateTime,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ValidateResponse {
    Invalid(InvalidValidateResponse),
    Valid(ValidValidateResponse),
}

impl ValidateResponse {
    pub fn validate(self) -> Result<ValidValidateResponse, Error> {
        match self {
            ValidateResponse::Valid(response) => {
                if response.valid {
                    Ok(response)
                } else {
                    Err(Error {
                        kind: ErrorKind::NotAuthenticated,
                        message: "Authentication error".to_string(),
                    })
                }
            }
            ValidateResponse::Invalid(response) => Err(Error {
                kind: ErrorKind::NotAuthenticated,
                message: response.error.message,
            }),
        }
    }
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

impl Identified for VirtualEntity {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sensor {
    pub protocol_id: String,
    pub resource_type_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Protocol {
    pub protocol: String,
    pub sensors: Vec<Sensor>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceType {
    #[serde(rename(deserialize = "deviceTypeId"))]
    pub id: String,
    pub description: Option<String>,
    pub active: bool,
    pub protocol: Protocol,
    #[serde(default)]
    pub configuration: serde_json::Value,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Identified for DeviceType {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSensor {
    pub protocol_id: String,
    pub resource_id: String,
    pub resource_type_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProtocol {
    pub protocol: String,
    pub sensors: Vec<DeviceSensor>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(rename(deserialize = "deviceId"))]
    pub id: String,
    pub description: Option<String>,
    pub active: bool,
    pub hardware_id: String,
    pub device_type_id: String,
    pub owner_id: String,
    pub hardware_id_names: Vec<String>,
    pub hardware_ids: HashMap<String, String>,
    pub parent_hardware_id: Vec<String>,
    pub tags: Vec<String>,
    pub protocol: DeviceProtocol,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Identified for Device {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceResourceTypeInfo {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub unit: Option<String>,
    pub range: Option<String>,
    pub is_cost: Option<bool>,
    pub method: Option<String>,
}

impl From<String> for DataSourceResourceTypeInfo {
    fn from(val: String) -> DataSourceResourceTypeInfo {
        DataSourceResourceTypeInfo {
            data_type: Some(val),
            unit: None,
            range: None,
            is_cost: None,
            method: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub field_name: String,
    pub datatype: String,
    pub negative: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub sampling: String,
    #[serde(default)]
    pub start: serde_json::Value,
    pub fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResourceType {
    #[serde(rename(deserialize = "resourceTypeId"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub label: Option<String>,
    pub active: bool,
    pub classifier: Option<String>,
    pub base_unit: Option<String>,
    pub data_source_type: String,
    #[serde(default, deserialize_with = "ds_type_info_deserializer")]
    pub data_source_resource_type_info: Option<DataSourceResourceTypeInfo>,
    #[serde(default)]
    pub units: HashMap<String, String>,
    pub storage: Vec<Storage>,
}

impl Identified for ResourceType {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename(deserialize = "resourceId"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub label: Option<String>,
    pub active: bool,
    #[serde(rename(deserialize = "resourceTypeId"))]
    pub type_id: String,
    pub owner_id: String,
    pub classifier: Option<String>,
    pub base_unit: Option<String>,
    pub data_source_type: String,
    #[serde(default, deserialize_with = "ds_type_info_deserializer")]
    pub data_source_resource_type_info: Option<DataSourceResourceTypeInfo>,
    pub data_source_unit_info: serde_json::Value,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Identified for Resource {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Debug)]
pub struct Reading {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    pub value: f32,
}

type ReadingTuple = (i64, f32);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReadingsResponse {
    pub data: Vec<ReadingTuple>,
}

/// The API endpoint.
///
/// Normally a non-default endpoint would only be useful for testing purposes.
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
pub struct GlowmarktApi {
    pub token: String,
    endpoint: GlowmarktEndpoint,
    client: Client,
}

impl GlowmarktApi {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_owned(),
            endpoint: Default::default(),
            client: Client::new(),
        }
    }

    /// Authenticates with the default Glowmarkt API endpoint.
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
    /// Authenticate against an endpoint.
    pub async fn auth(
        endpoint: GlowmarktEndpoint,
        username: &str,
        password: &str,
    ) -> Result<GlowmarktApi, Error> {
        let client = Client::new();
        let request = client.post(endpoint.url("auth")).json(&AuthRequest {
            username: username.to_owned(),
            password: password.to_owned(),
        });

        let response = endpoint
            .api_call::<AuthResponse>(&client, request)
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
            .request::<ValidateResponse>()
            .await
            .and_then(|r| r.validate())?;

        log::debug!("Authenticated with API until {}", iso(response.expiry));

        Ok(true)
    }
}

/// [Device Management System](https://api.glowmarkt.com/api-docs/v0-1/dmssys/#/)
impl GlowmarktApi {
    /// Retrieves all of the known device types.
    pub async fn device_types(&self) -> Result<HashMap<String, DeviceType>, Error> {
        self.get_request("devicetype")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves all of the devices registered for an account.
    pub async fn devices(&self) -> Result<HashMap<String, Device>, Error> {
        self.get_request("device").request().await.map(build_map)
    }

    /// Retrieves a single device.
    pub async fn device(&self, id: &str) -> Result<Option<Device>, Error> {
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
    pub async fn virtual_entities(&self) -> Result<HashMap<String, VirtualEntity>, Error> {
        self.get_request("virtualentity")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves a single virtual entity by ID.
    pub async fn virtual_entity(&self, entity_id: &str) -> Result<Option<VirtualEntity>, Error> {
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
    pub async fn resource_types(&self) -> Result<HashMap<String, ResourceType>, Error> {
        self.get_request("resourcetype")
            .request()
            .await
            .map(build_map)
    }

    /// Retrieves all resources.
    pub async fn resources(&self) -> Result<HashMap<String, Resource>, Error> {
        self.get_request("resource").request().await.map(build_map)
    }

    /// Retrieves a single resource by ID.
    pub async fn resource(&self, resource_id: &str) -> Result<Option<Resource>, Error> {
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
            .request::<ReadingsResponse>()
            .await?;

        Ok(readings
            .data
            .into_iter()
            .map(|(timestamp, value)| Reading {
                start: OffsetDateTime::from_unix_timestamp(timestamp).unwrap(),
                value,
            })
            .collect())
    }
}

fn ds_type_info_deserializer<'de, D>(
    deserializer: D,
) -> Result<Option<DataSourceResourceTypeInfo>, D::Error>
where
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct;

    impl<'de> Visitor<'de> for StringOrStruct {
        type Value = Option<DataSourceResourceTypeInfo>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or object")
        }

        fn visit_none<E>(self) -> Result<Option<DataSourceResourceTypeInfo>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<DataSourceResourceTypeInfo>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_owned().into()))
        }

        fn visit_string<E>(self, value: String) -> Result<Option<DataSourceResourceTypeInfo>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.into()))
        }

        fn visit_map<M>(self, map: M) -> Result<Option<DataSourceResourceTypeInfo>, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
        }
    }

    deserializer.deserialize_any(StringOrStruct)
}
