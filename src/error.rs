use thiserror::Error;

#[derive(Error, Debug)]
pub enum MainError {
    #[error("No context found")]
    NoContext,

    #[error("No namespace found")]
    NoNamespace,

    #[error("No service found in namespace '{0}'")]
    NoService(String),

    #[error("No ports selected")]
    NoPorts,

    #[error("No valid selection")]
    InvalidSelection(#[from] dialoguer::Error),

    #[error(transparent)]
    KubectlFailed(#[from] KubectlError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    CtrlC(#[from] ctrlc::Error),
}

#[derive(Error, Debug)]
pub enum KubectlError {
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("Command failed")]
    CommandFailed,

    #[error(transparent)]
    ParseOutput(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
