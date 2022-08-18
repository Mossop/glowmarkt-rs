pub use api::GlowmarktEndpoint;
use time::OffsetDateTime;

mod api;
mod error;

pub use api::{Reading, ReadingPeriod, VirtualEntityDetail as VirtualEntity};
pub use error::Error;

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

    pub async fn readings(
        &self,
        resource_id: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        period: ReadingPeriod,
    ) -> Result<Vec<Reading>, Error> {
        self.endpoint
            .readings(&self.token, resource_id, start, end, period)
            .await
    }
}
