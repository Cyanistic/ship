use std::{
    env, fs,
    future::Future,
    io::{self, IsTerminal},
    panic,
    path::PathBuf,
    process,
    sync::Arc,
};

use ship_core::{LogChange, LogFilter, prelude::*};
use tracing::Level;
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{self, format::FmtSpan, time::ChronoLocal},
    prelude::*,
    reload,
};

use crate::runlog::{self, RunLog};

pub enum Role {
    Server {
        port: u16,
        log_file: Option<PathBuf>,
    },
    Client,
    Command,
}

pub struct Diagnostics {
    log: Option<Arc<RunLog>>,
    filter: reload::Handle<EnvFilter, Registry>,
    startup: String,
    // Dropped after the runtime shuts down to finish the timeline's JSON.
    _timeline: Option<tracing_chrome::FlushGuard>,
}

impl Diagnostics {
    pub fn server_log(
        &self,
    ) -> impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static {
        let log = self.log.clone();
        let filter = self.filter.clone();
        let startup = self.startup.clone();
        move |change| {
            if let Some(change) = change {
                let next = match change {
                    LogChange::Set(value) => value,
                    LogChange::Reset => startup.clone(),
                };
                let next = EnvFilter::try_new(next)
                    .map_err(|error| err!(Validation, "invalid log filter", @external: error))?;
                filter.reload(next).map_err(
                    |error| err!(Internal, "cannot reload log filter", @external: error),
                )?;
            }
            Ok(LogFilter {
                active: filter
                    .with_current(ToString::to_string)
                    .map_err(|error| err!(Internal, "cannot read log filter", @external: error))?,
                startup: startup.clone(),
                file: log
                    .as_ref()
                    .ok_or_else(|| err!(Internal, "server has no run log"))?
                    .path()
                    .to_string_lossy()
                    .into_owned(),
            })
        }
    }
}

pub fn init(role: Role) -> Result<Diagnostics> {
    let filter = EnvFilter::try_new(startup_filter()?)
        .map_err(|error| err!(Configuration, "invalid RUST_LOG filter; expected comma-separated target=level directives (error, warn, info, debug, trace, off)", @external: error))?;
    let trace_role = match &role {
        Role::Server { .. } => Some("server"),
        Role::Client => Some("client"),
        Role::Command => None,
    };
    let stderr = matches!(role, Role::Command | Role::Server { log_file: None, .. });
    let log = match role {
        Role::Server { port, log_file } => Some(match log_file {
            Some(path) => RunLog::resume(&path)?,
            None => RunLog::start(
                &runlog::logs_dir()?.join(port.to_string()),
                runlog::stem(&chrono::Local::now()),
            )?,
        }),
        Role::Client => Some(RunLog::start(
            &runlog::logs_dir()?.join("client"),
            format!("{}-{}", runlog::stem(&chrono::Local::now()), process::id()),
        )?),
        Role::Command => None,
    };
    let startup = filter.to_string();
    let (filter, handle) = reload::Layer::new(filter);
    let log = log.map(Arc::new);
    let file = log.clone().map(|log| {
        fmt::layer()
            .with_timer(ChronoLocal::rfc_3339())
            .with_span_events(FmtSpan::NEW)
            .with_ansi(false)
            .with_writer(log)
    });
    let terminal = stderr.then(|| {
        fmt::layer()
            .with_timer(ChronoLocal::rfc_3339())
            .with_span_events(FmtSpan::NEW)
            .with_ansi(io::stderr().is_terminal())
            .with_writer(io::stderr)
    });
    let (timeline, guard) = if let (Some(dir), Some(role)) = (env::var_os("SHIP_TRACE"), trace_role)
    {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).map_err(|error| {
                err!(Io, "cannot create timeline directory {}", dir.display(), @external: error)
            })?;
        let path = dir.join(format!("{role}-{}.json", process::id()));
        let writer = fs::File::create(&path).map_err(
            |error| err!(Io, "cannot create timeline {}", path.display(), @external: error),
        )?;
        let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
            .writer(writer)
            .include_args(true)
            .build();
        let filter = EnvFilter::new("warn,ship=debug,ratatui_ghostty=debug,kameo=info");
        (Some(layer.with_filter(filter)), Some(guard))
    } else {
        (None, None)
    };
    tracing_subscriber::registry()
        .with(Layer::and_then(file, terminal).with_filter(filter))
        .with(timeline)
        .try_init()
        .map_err(|error| err!(Internal, "cannot initialize diagnostics", @external: error))?;
    panic_hook();
    Ok(Diagnostics {
        log,
        filter: handle,
        startup,
        _timeline: guard,
    })
}

fn startup_filter() -> Result<String> {
    match env::var("RUST_LOG") {
        Ok(value) => return Ok(value),
        Err(env::VarError::NotPresent) => {}
        Err(_) => return Err(err!(Configuration, "RUST_LOG must be valid Unicode")),
    }
    let level = match env::var("SHIP_LOG") {
        Ok(value) => value.parse::<Level>().map_err(|_| {
            err!(
                Configuration,
                "SHIP_LOG must be one of: error, warn, info, debug, trace"
            )
        })?,
        Err(env::VarError::NotPresent) => Level::INFO,
        Err(_) => {
            return Err(err!(
                Configuration,
                "SHIP_LOG must be one of: error, warn, info, debug, trace"
            ));
        }
    };
    Ok(level_filter(level))
}

pub fn level_filter(level: Level) -> String {
    let actor = if level >= Level::DEBUG {
        "info"
    } else {
        "warn"
    };
    format!(
        "warn,ship={},ratatui_ghostty={},kameo={actor}",
        level.as_str().to_ascii_lowercase(),
        level.as_str().to_ascii_lowercase()
    )
}

fn panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "process panicked");
        previous(info);
    }));
}

/// Resolves on SIGINT or SIGTERM.
#[cfg(unix)]
pub fn shutdown() -> Result<impl Future<Output = Result<()>> + Send + 'static> {
    use tokio::signal::unix::{SignalKind, signal};

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
