pub use api::GlowmarktEndpoint;
use time::OffsetDateTime;

mod api;
mod error;

pub use error::Error;
pub type VirtualEntity = api::VirtualEntityDetail;

pub fn iso(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        dt.year(),
        dt.month() as u8 + 1,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

#[derive(Debug, Clone)]
pub struct Glowmarkt {
    endpoint: GlowmarktEndpoint,
    token: String,
}

impl Glowmarkt {
    pub async fn authenticate(username: String, password: String) -> Result<Self, Error> {
        GlowmarktEndpoint::default()
            .authenticate(username, password)
            .await
    }

    pub async fn virtual_entities(&self) -> Result<Vec<VirtualEntity>, Error> {
        let response = self.endpoint.virtual_entities(&self.token).await?;
        let mut entities = Vec::new();

        for entity in response {
            let entity = self
                .endpoint
                .virtual_entity(&self.token, &entity.id)
                .await?;
            entities.push(entity);
        }

        log::trace!("Saw entities: {:?}", entities);
        Ok(entities)
    }
}
