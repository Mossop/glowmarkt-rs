use std::{collections::HashMap, fmt};

use glowmarkt::{Device, Resource};
use time::{OffsetDateTime, UtcOffset};

pub struct Measurement {
    pub id: String,
    pub timestamp: i128,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, f64>,
}

impl Measurement {
    pub fn new(id: &str, timestamp: OffsetDateTime, tags: HashMap<String, String>) -> Self {
        Measurement {
            id: id.to_owned(),
            timestamp: timestamp.to_offset(UtcOffset::UTC).unix_timestamp_nanos(),
            tags,
            fields: HashMap::new(),
        }
    }

    pub fn add_field(&mut self, key: &str, value: f64) {
        assert!(value.is_finite());

        self.fields.insert(key.to_owned(), value);
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(!self.fields.is_empty());

        let tags = self
            .tags
            .iter()
            .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
            .collect::<Vec<String>>();

        let fields = self
            .fields
            .iter()
            .map(|(k, v)| format!("{}={}", escape(k), v))
            .collect::<Vec<String>>();

        if !tags.is_empty() {
            f.pad(&format!(
                "{},{} {} {}",
                self.id,
                tags.join(","),
                fields.join(","),
                self.timestamp
            ))
        } else {
            f.pad(&format!(
                "{} {} {}",
                self.id,
                fields.join(","),
                self.timestamp
            ))
        }
    }
}

pub fn tags_for_device(device: &Device) -> HashMap<String, String> {
    let mut tags = HashMap::new();
    tags.insert("device-id".to_string(), device.id.clone());
    if let Some(ref description) = device.description {
        tags.insert("device".to_string(), description.clone());
    }
    tags
}

pub fn tags_for_resource(
    tags: &HashMap<String, String>,
    resource: &Resource,
) -> HashMap<String, String> {
    let mut tags = tags.clone();
    tags.insert("resource-id".to_string(), resource.id.clone());
    tags.insert("resource".to_string(), resource.name.clone());
    if let Some(ref classifier) = resource.classifier {
        tags.insert("classifier".to_string(), classifier.clone());
    }
    if let Some(ref unit) = resource.base_unit {
        tags.insert("unit".to_string(), unit.clone());
    }
    tags
}

fn escape(tag: &str) -> String {
    tag.replace(' ', "\\ ").replace(',', "\\,")
}
