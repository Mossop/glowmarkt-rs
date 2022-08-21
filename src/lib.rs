pub use api::GlowmarktApi;
use time::OffsetDateTime;

pub mod api;
mod error;

pub use api::{Reading, ReadingPeriod, VirtualEntityDetail};
pub use error::Error;

#[derive(Debug, Clone)]
pub struct Glowmarkt {
    api: GlowmarktApi,
}

impl Glowmarkt {
    pub async fn authenticate(username: String, password: String) -> Result<Self, Error> {
        let api = GlowmarktApi::authenticate(username, password).await?;
        Ok(Glowmarkt { api })
    }

    pub async fn virtual_entities(&self) -> Result<Vec<VirtualEntityDetail>, Error> {
        let response = self.api.virtual_entities().await?;
        let mut entities = Vec::new();

        for entity in response {
            let entity = self.api.virtual_entity(&entity.id).await?;
            entities.push(entity);
        }

        log::trace!("Saw entities: {:?}", entities);
        Ok(entities)
    }

    pub async fn readings(
        &self,
        resource_id: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        period: ReadingPeriod,
    ) -> Result<Vec<Reading>, Error> {
        self.api.readings(resource_id, start, end, period).await
    }
}
