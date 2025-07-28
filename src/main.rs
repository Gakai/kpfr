mod error;
mod kubectl;
mod model;
mod selection;

use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{fs, thread};

use dialoguer::MultiSelect;
use dialoguer::{FuzzySelect, Input, theme::Theme};
use indicatif::ProgressBar;

use crate::error::MainError;
use crate::kubectl::{context, namespace, service};
use crate::model::{Namespace, Service};
use crate::selection::{DefaultSelections, Selection};

type Result<T> = std::result::Result<T, MainError>;

fn preselect_context(theme: &dyn Theme) -> Result<()> {
    let contexts = context::get()?;
    if contexts.is_empty() {
        return Err(MainError::NoContext);
    }
    let current_ctx = context::current().unwrap_or(String::from(""));

    if contexts.len() > 1 {
        let mut prompt = dialoguer::FuzzySelect::with_theme(theme)
            .with_prompt("Select context")
            .items(&contexts);
        let default_idx = contexts.iter().position(|ctx| current_ctx.eq(ctx));
        if let Some(i) = default_idx {
            prompt = prompt.default(i);
        }
        let selected_idx = prompt.interact()?;

        context::set(&contexts[selected_idx]).expect("Failed to select context");
    }
    Ok(())
}

fn select_namespace(theme: &dyn Theme, default: Option<String>) -> Result<Namespace> {
    // Loading namespaces
    let bar = ProgressBar::new_spinner().with_message("Getting available namespaces...");
    bar.enable_steady_tick(Duration::from_millis(100));
    let namespaces = namespace::get()?;
    bar.finish_and_clear();

    // Ensure at least one is available
    if namespaces.is_empty() {
        return Err(MainError::NoNamespace);
    }

    // Show selection if more than one namespace
    if namespaces.len() > 1 {
        let mut prompt = FuzzySelect::with_theme(theme)
            .with_prompt("Select namespace")
            .items(&namespaces);
        let default_idx =
            default.and_then(|d| namespaces.iter().position(|ns| ns.metadata.name.eq(&d)));
        if let Some(i) = default_idx {
            prompt = prompt.default(i);
        }
        let selected_idx = prompt.interact()?;
        Ok(namespaces[selected_idx].to_owned())
    } else {
        // NOTE: Checked previously that at least one exists
        Ok(namespaces[0].to_owned())
    }
}

fn select_service(
    theme: &dyn Theme,
    namespace: &Namespace,
    default: Option<String>,
) -> Result<Service> {
    // Loading services of given namespace
    let spinner = ProgressBar::new_spinner().with_message(format!(
        "Reading services of {}...",
        namespace.metadata.name
    ));
    spinner.enable_steady_tick(Duration::from_millis(100));
    let services = service::get(&namespace.metadata.name)?;
    spinner.finish_and_clear();

    if services.is_empty() {
        return Err(MainError::NoService(namespace.metadata.name.to_owned()));
    }

    if services.len() > 1 {
        let mut prompt = FuzzySelect::with_theme(theme)
            .with_prompt("Select service")
            .items(&services);
        let default_idx =
            default.and_then(|d| services.iter().position(|s| s.metadata.name.eq(&d)));
        if let Some(i) = default_idx {
            prompt = prompt.default(i);
        }
        let selected_idx = prompt.interact()?;
        Ok(services[selected_idx].to_owned())
    } else {
        // NOTE: Checked previously that at least one exists
        Ok(services[0].to_owned())
    }
}

fn select_remote_ports(
    theme: &dyn Theme,
    service: &Service,
    default_ports: &HashMap<u16, u16>,
) -> Result<Vec<u16>> {
    let default_ports = default_ports
        .keys()
        .map(|k| k.to_owned())
        .collect::<Vec<_>>();
    let port_items = service.spec.ports.clone();
    let ports = port_items
        .iter()
        .map(|p| (p.port, default_ports.contains(&p.port)))
        .collect::<Vec<_>>();

    if ports.len() == 1 {
        let selections = MultiSelect::with_theme(theme)
            .items_checked(&ports)
            .interact()?;
        Ok(selections
            .iter()
            .map(|s| port_items[*s].port)
            .collect::<Vec<_>>())
    } else {
        Ok(ports.iter().map(|p| p.0).collect())
    }
}

fn select_local_ports(
    theme: &dyn Theme,
    selected_ports: &Vec<u16>,
    service_ports: &HashMap<u16, u16>,
) -> Result<HashMap<u16, u16>> {
    let mut ports = HashMap::new();
    for port in selected_ports {
        let mut prompt = Input::<u16>::with_theme(theme)
            .with_prompt(format!("Forward container port {} to local port:", port));
        if service_ports.contains_key(port) {
            let default_local_port = service_ports[port];
            prompt = prompt.default(default_local_port);
        }
        let local_port = prompt.interact()?;
        ports.entry(*port).insert_entry(local_port);
    }
    Ok(ports)
}

fn fail(e: MainError) -> ExitCode {
    eprintln!("{e}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let config_dir = dirs::config_dir().unwrap().join(env!("CARGO_PKG_NAME"));
    if !fs::exists(&config_dir).unwrap() {
        eprintln!(
            "Creating config directory {}",
            config_dir.to_str().unwrap_or("<unknown>")
        );
        fs::create_dir_all(&config_dir).unwrap();
    }
    let filename = config_dir.join("config.json");
    let defaults = DefaultSelections::read(&filename);

    // Select context if more than one are available
    if let Err(e) = preselect_context(&theme) {
        return fail(e);
    }

    // Select namespace
    let default_namespace = defaults.clone().and_then(|d| d.namespace);
    let namespace = match select_namespace(&theme, default_namespace) {
        Ok(n) => n,
        Err(e) => return fail(e),
    };
    let selection = Selection::from_defaults(&namespace, &defaults);

    // Select service
    let default_service = defaults.clone().and_then(|d| d.last_service);
    let service = match select_service(&theme, &namespace, default_service) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let mut selection = selection.set_last_service(&service);

    // Get default ports for the selected service
    let default_ports = selection.ports_for(&service);

    // Select remote ports from service
    let remote_ports = match select_remote_ports(&theme, &service, default_ports) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };

    // Abort if no ports selected
    if remote_ports.is_empty() {
        selection.save(&filename).unwrap();
        eprintln!("{}", MainError::NoPorts);
        return ExitCode::FAILURE;
    }

    // Decide which local ports to map to
    let ports_mapping = match select_local_ports(&theme, &remote_ports, default_ports) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };

    // Save selections to file
    selection
        .ports
        .entry(service.metadata.name.clone())
        .insert_entry(ports_mapping);
    selection.save(&filename).unwrap();

    // Abort if no ports selected
    if remote_ports.is_empty() {
        eprintln!("{}", MainError::NoPorts);
        return ExitCode::FAILURE;
    }

    // Forward ports (keeps running in subprocess)
    let ports = selection.ports.get(&service.metadata.name).unwrap();
    let running = Arc::new(AtomicBool::new(true));
    let mut forward_process = match kubectl::forward_ports(&namespace, &service, ports)
        .map_err(MainError::KubectlFailed)
    {
        Ok(fp) => fp,
        Err(e) => return fail(e),
    };

    // Add Ctrl-C handler to cancel/finish the port-forwarding
    let r1 = Arc::clone(&running);
    if let Err(e) = ctrlc::set_handler(move || {
        forward_process.kill().unwrap();
        forward_process.wait().unwrap();
        eprintln!("\nPort-forward terminated successfully.");
        r1.store(false, Ordering::Relaxed);
    })
    .map_err(MainError::CtrlC)
    {
        return fail(e);
    }

    // Keep the main process running while forwarding process runs
    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }

    ExitCode::SUCCESS
}
