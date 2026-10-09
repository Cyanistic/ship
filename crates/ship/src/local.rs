use std::{
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use ship_client::Client;
use ship_core::{DEFAULT_PORT, prelude::*};
use tokio::time::{Instant, sleep};

/// Reuse the default server, or launch one with the starter tab. True when
/// this call launched it, even if a concurrent launcher's server won the bind.
/// The launched server reads `config`.
pub async fn default_health(client: &Client, config: &Path) -> Result<bool> {
    match crate::health(client).await {
        Ok(_) => return Ok(false),
        Err(error) if *error.code() == ErrorCode::ConnectionRefused => {}
        Err(error) => return Err(error),
    }

    let executable = std::env::current_exe()
        .map_err(|error| err!(Io, "cannot resolve current executable", @external: error))?;
    let directory = std::env::temp_dir();
    let temporary = tempfile::Builder::new().prefix("ship-server-").suffix(".log")
        .tempfile_in(&directory)
        .map_err(|error| err!(Io, "cannot create private server log in {}; no file was created", directory.display(), @external: error))?;
    // NamedTempFile uses exclusive creation and mode 0600 on Unix. Keep it even on failure.
    let (file, log) = temporary.keep().map_err(
        |error| err!(Io, "cannot retain server log in {}", directory.display(), @external: error),
    )?;
    let stderr = file
        .try_clone()
        .map_err(|error| err!(Io, "cannot clone server log handle", @external: error))
        .context(format!("launch failed; retained log {}", log.display()))?;
    let mut child = Command::new(executable)
        .args([
            "server",
            "--port",
            &DEFAULT_PORT.to_string(),
            "--starter",
            "--background-child",
        ])
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| err!(Io, "cannot spawn local server", @external: error))
        .context(format!("launch failed; retained log {}", log.display()))?;

    let deadline = Instant::now() + Duration::from_secs(5);
    let readiness = async {
        loop {
            if Instant::now() >= deadline {
                return Err(err!(
                    Network,
                    "local server readiness exceeded five seconds"
                ));
            }
            let exited = child
                .try_wait()
                .map_err(|error| err!(Io, "cannot inspect launched child", @external: error))?;
            // Even after child exit, this bounded final probe can accept a concurrent winner.
            match tokio::time::timeout_at(deadline, crate::health(client))
                .await
                .map_err(|_| err!(Network, "local server readiness exceeded five seconds"))?
            {
                Ok(response) => return Ok(response),
                Err(error) if *error.code() == ErrorCode::ConnectionRefused => {
                    if let Some(status) = exited {
                        return Err(err!(
                            Internal,
                            "launched child exited with {} before compatible health",
                            status
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
            sleep(
                Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
            )
            .await;
        }
    }
    .await;

    let error = match readiness {
        Ok(_) => {
            tracing::info!(pid = child.id(), log = %log.display(), "local server launch ready (a concurrent server may have won)");
            return Ok(true);
        }
        Err(error) => error,
    };
    let context = format!(
        "local startup failed (child PID {}, retained log {})",
        child.id(),
        log.display()
    );
    let cleanup = match child.try_wait() {
        Ok(None) => child
            .kill()
            .map_err(|error| err!(Io, "cannot terminate owned failed child", @external: error)),
        Ok(Some(_)) => Ok(()),
        Err(error) => Err(err!(Io, "cannot inspect owned child during cleanup", @external: error)),
    }
    .and_then(|()| {
        child
            .wait()
            .map(|_| ())
            .map_err(|error| err!(Io, "cannot reap owned child", @external: error))
    });
    match cleanup {
        Ok(()) => Err(error.context(context)),
        Err(cleanup) => Err(cleanup.context(format!("{context}; original failure: {error}"))),
    }
}
