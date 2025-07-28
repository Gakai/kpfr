use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KubectlList<T> {
    pub items: Vec<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Namespace {
    pub metadata: Metadata,
}
impl Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.metadata.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}
impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.metadata.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceSpec {
    pub ports: Vec<Port>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub port: u16,
}
