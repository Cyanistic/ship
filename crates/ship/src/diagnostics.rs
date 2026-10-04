use std::{future::Future, io::IsTerminal};

use ship_core::prelude::*;
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::EnvFilter;

pub fn init() -> Result<()> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)
            .map_err(|error| err!(Configuration, "invalid RUST_LOG filter", @external: error))?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(_) => return Err(err!(Configuration, "RUST_LOG must be valid Unicode")),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .try_init()
        .map_err(|error| err!(Internal, "cannot initialize diagnostics", @external: error))
}

pub fn shutdown() -> Result<impl Future<Output = Result<()>> + Send + 'static> {
    // Register both signals before starting the listener; registration errors are fatal.
    let mut interrupt = signal(SignalKind::interrupt())
        .map_err(|error| err!(Io, "cannot register SIGINT", @external: error))?;
    let mut terminate = signal(SignalKind::terminate())
        .map_err(|error| err!(Io, "cannot register SIGTERM", @external: error))?;
    Ok(async move {
        let received = tokio::select! {
            value = interrupt.recv() => value,
            value = terminate.recv() => value,
        };
        received.ok_or_else(|| err!(Io, "shutdown signal stream closed"))?;
        tracing::info!("shutdown requested");
        Ok(())
    })
}
