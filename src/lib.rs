use reqwest::Client;
use serde::{Deserialize, Serialize};

mod api;
mod error;

pub use error::Error;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Glowmarkt {
    pub account_id: String,
    pub token: String,
    pub expiry: u64,
}

impl Glowmarkt {
    pub async fn authenticate(username: String, password: String) -> Result<Self, Error> {
        let client = Client::new();

        let response = api::auth(&client, &api::AuthRequest { username, password }).await?;

        if !response.valid {
            return Error::err("Authentication error");
        }

        Ok(Self {
            account_id: response.account_id,
            token: response.token,
            expiry: response.exp,
        })
    }
}
