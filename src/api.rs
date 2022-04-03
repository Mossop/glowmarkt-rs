use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::Error;

const BASE_URL: &str = "https://api.glowmarkt.com/api/v0-1";
const APPLICATION_ID: &str = "b0f1b774-a586-4f72-9edd-27ead8aa7a8d";

#[derive(Serialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub valid: bool,
    pub account_id: String,
    pub token: String,
    pub exp: u64,
}

async fn api_call<T>(builder: RequestBuilder) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let response = builder
        .header("Content-Type", "application/json")
        .header("applicationId", APPLICATION_ID)
        .send()
        .await?;

    if !response.status().is_success() {
        return Error::err(format!(
            "API returned unexpected response: {}",
            response.status()
        ));
    }

    response.json::<T>().await.map_err(|e| e.to_string().into())
}

pub async fn auth(client: &Client, request: &AuthRequest) -> Result<AuthResponse, Error> {
    api_call(client.post(format!("{}/auth", BASE_URL)).json(request))
        .await
        .map_err(|e| Error::from(format!("Error authenticating: {}", e)))
}
