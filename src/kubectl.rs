use std::collections::HashMap;
use std::process::{Child, Command};

use crate::error::KubectlError;
use crate::model::{Namespace, Service};

const KUBECTL: &str = "kubectl";

type Result<T> = std::result::Result<T, KubectlError>;

pub mod context {
    use std::process::Command;

    use super::*;

    const KUBECTL: &str = "kubectl";

    pub fn current() -> Result<String> {
        let output = Command::new(KUBECTL)
            .args(["config", "current-context"])
            .output()?;
        if !output.status.success() {
            return Err(KubectlError::CommandFailed);
        }

        Ok(String::from_utf8(output.stdout)?.trim().into())
    }

    pub fn get() -> Result<Vec<String>> {
        let output = Command::new(KUBECTL)
            .args(["config", "get-contexts", "--output=name"])
            .output()?;
        if !output.status.success() {
            return Err(KubectlError::CommandFailed);
        }
        Ok(String::from_utf8(output.stdout)?
            .trim()
            .lines()
            .map(String::from)
            .collect::<Vec<_>>())
    }

    pub fn set(context: &str) -> Result<()> {
        let output = Command::new(KUBECTL)
            .args(["config", "use-context", context])
            .output()?;
        if !output.status.success() {
            Err(KubectlError::CommandFailed)
        } else {
            Ok(())
        }
    }
}

pub mod namespace {
    use std::process::Command;

    use super::*;
    use crate::model::{KubectlList, Namespace};

    const KUBECTL: &str = "kubectl";

    pub fn get() -> Result<Vec<Namespace>> {
        let output = Command::new(KUBECTL)
            .args(["get", "namespaces", "--output=json"])
            .output()?;

        if !output.status.success() {
            return Err(KubectlError::CommandFailed);
        }

        let output = String::from_utf8(output.stdout)?;

        Ok(serde_json::from_str::<KubectlList<Namespace>>(&output)?.items)
    }
}

pub mod service {
    use std::process::Command;

    use super::*;
    use crate::model::{KubectlList, Service};

    pub fn get(namespace: &str) -> Result<Vec<Service>> {
        let output = Command::new(KUBECTL)
            .args(["--namespace", namespace, "get", "services", "--output=json"])
            .output()?;

        if !output.status.success() {
            return Err(KubectlError::CommandFailed);
        }

        let output = String::from_utf8(output.stdout)?;

        Ok(serde_json::from_str::<KubectlList<Service>>(&output)?.items)
    }
}

#[allow(unused)]
pub fn forward_ports(
    namespace: &Namespace,
    service: &Service,
    ports: &HashMap<u16, u16>,
) -> Result<Child> {
    Ok(Command::new(KUBECTL)
        .args(
            [
                "--namespace".into(),
                namespace.to_string(),
                "port-forward".into(),
                format!("service/{}", service),
            ]
            .into_iter()
            .chain(
                ports
                    .iter()
                    .map(|(remote_port, local_port)| format!("{local_port}:{remote_port}")),
            ),
        )
        .spawn()?)
}
