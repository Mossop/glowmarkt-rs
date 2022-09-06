use std::{collections::BTreeMap, fmt};

use glowmarkt::{Device, Resource};
use time::{OffsetDateTime, UtcOffset};

pub struct Measurement {
    pub id: String,
    pub timestamp: i128,
    pub tags: BTreeMap<String, String>,
    pub fields: BTreeMap<String, f64>,
}

impl Measurement {
    pub fn new(id: &str, timestamp: OffsetDateTime, tags: BTreeMap<String, String>) -> Self {
        Measurement {
            id: id.to_owned(),
            timestamp: timestamp.to_offset(UtcOffset::UTC).unix_timestamp_nanos(),
            tags,
            fields: BTreeMap::new(),
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

pub fn add_tags_for_device(tags: &mut BTreeMap<String, String>, device: &Device) {
    tags.insert("device-id".to_string(), device.id.clone());
    if let Some(ref description) = device.description {
        tags.insert("device".to_string(), description.clone());
    }
    tags.insert("device-active".to_string(), device.active.to_string());
    tags.insert("hardware-id".to_string(), device.hardware_id.to_string());
    for (k, v) in device.hardware_ids.iter() {
        tags.insert(k.clone(), v.clone());
    }
}

pub fn add_tags_for_resource(tags: &mut BTreeMap<String, String>, resource: &Resource) {
    tags.insert("resource-id".to_string(), resource.id.clone());
    tags.insert("resource".to_string(), resource.name.clone());
    tags.insert("resource-active".to_string(), resource.active.to_string());

    if let Some(ref classifier) = resource.classifier {
        tags.insert("classifier".to_string(), classifier.clone());
    }

    if let Some(ref unit) = resource.base_unit {
        tags.insert("unit".to_string(), unit.clone());
    }

    if let Some(ref classifier) = resource.classifier {
        if let Some(class) = classifier.split('.').next() {
            tags.insert("class".to_string(), class.to_string());
        }
    }
}

pub fn field_for_classifier(classifier: &Option<String>) -> &str {
    if let Some(classifier) = classifier {
        classifier.split('.').last().unwrap()
    } else {
        "value"
    }
}

fn escape(tag: &str) -> String {
    tag.replace(' ', "\\ ").replace(',', "\\,")
}
