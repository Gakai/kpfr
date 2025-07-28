#![allow(unused)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Result, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Namespace, Service};

#[derive(Serialize, Debug, Clone)]
pub struct Selection {
    pub namespace: String,
    pub ports: HashMap<String, HashMap<u16, u16>>,
}
impl Selection {
    pub fn from_defaults(namespace: &Namespace, defaults: &Option<DefaultSelections>) -> Self {
        Self {
            namespace: namespace.metadata.name.to_owned(),
            ports: defaults
                .as_ref()
                .and_then(|d| d.ports.clone())
                .unwrap_or_default(),
        }
    }

    pub fn set_last_service(self, service: &Service) -> SelectionWithService {
        SelectionWithService {
            last_service: service.metadata.name.to_owned(),
            namespace: self.namespace,
            ports: self.ports,
        }
    }

    pub fn save<P: AsRef<Path>>(&self, filename: &P) -> Result<()> {
        let data = serde_json::to_string_pretty(self).unwrap();
        File::create(filename).unwrap().write_all(data.as_bytes())
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SelectionWithService {
    pub namespace: String,
    pub ports: HashMap<String, HashMap<u16, u16>>,
    pub last_service: String,
}
impl SelectionWithService {
    pub fn save<P: AsRef<Path>>(&self, filename: &P) -> Result<()> {
        let data = serde_json::to_string_pretty(self).unwrap();
        File::create(filename).unwrap().write_all(data.as_bytes())
    }

    pub fn set_last_service(self, service: &Service) -> Self {
        Self {
            last_service: service.metadata.name.to_owned(),
            ..self
        }
    }

    pub fn ports_for(&mut self, service: &Service) -> &mut HashMap<u16, u16> {
        self.ports.entry(service.metadata.name.clone()).or_default()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSelections {
    pub namespace: Option<String>,
    pub last_service: Option<String>,
    pub ports: Option<HashMap<String, HashMap<u16, u16>>>,
}
impl DefaultSelections {
    pub fn read<P: AsRef<Path>>(filename: &P) -> Option<Self> {
        let file = File::open(filename).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }
}
