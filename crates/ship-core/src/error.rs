use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::panic::Location;
use std::result;

#[cfg(feature = "axum")]
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
#[cfg(feature = "kameo")]
use kameo::error::SendError;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

pub type Result<T, E = AppError> = result::Result<T, E>;

/// Diagnostic categories. HTTP response policy remains server-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", content = "data")]
pub enum ErrorCode {
    Validation,
    NotFound,
    Conflict,
    Configuration,
    Network,
    ConnectionRefused,
    RateLimited,
    UpstreamHttpStatus(u16),
    Internal,
    Serialization,
    Unauthorized,
    Io,
    /// A well-formed request that would break the tab/pane structure.
    InvalidStructure,
    /// Server machinery, such as the state actor, cannot take the request.
    Unavailable,
}

impl ErrorCode {
    /// Suggested status, without forwarding arbitrary upstream statuses.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Validation => 400,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::Configuration | Self::Internal | Self::Serialization | Self::Io => 500,
            Self::Network | Self::ConnectionRefused | Self::UpstreamHttpStatus(_) => 502,
            Self::RateLimited => 429,
            Self::Unauthorized => 401,
            Self::InvalidStructure => 422,
            Self::Unavailable => 503,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum AppError {
    Internal(InternalError),
    External(ExternalError),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InternalError {
    pub message: Cow<'static, str>,
    #[serde(rename = "type")]
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(no_recursion)]
    pub caused_by: Option<Box<AppError>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ExternalError {
    pub code: ErrorCode,
    /// A lossy diagnostic snapshot, not a retained foreign error object.
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AppError {
    pub fn internal(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::Internal(InternalError {
            message: message.into(),
            code,
            data: None,
            caused_by: None,
        })
    }

    pub fn external(code: ErrorCode, error: impl fmt::Display) -> Self {
        Self::External(ExternalError {
            code,
            error: error.to_string(),
            data: None,
        })
    }

    pub fn code(&self) -> &ErrorCode {
        match self {
            Self::Internal(error) => &error.code,
            Self::External(error) => &error.code,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        match &mut self {
            Self::Internal(error) => error.data = Some(data),
            Self::External(error) => error.data = Some(data),
        }
        self
    }

    /// Attach a cause to an internal error; external snapshots have no cause slot.
    pub fn with_cause(mut self, cause: AppError) -> Self {
        if let Self::Internal(error) = &mut self {
            error.caused_by = Some(Box::new(cause));
        }
        self
    }

    /// Preserve the category and complete original error beneath a new message.
    pub fn context(self, message: impl Into<Cow<'static, str>>) -> Self {
        Self::internal(*self.code(), message).with_cause(self)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(error) => {
                write!(f, "{}", error.message)?;
                if let Some(cause) = &error.caused_by {
                    write!(f, ": {cause}")?;
                }
                Ok(())
            }
            Self::External(error) => write!(f, "{}", error.error),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Internal(error) => error.caused_by.as_deref().map(|cause| cause as &dyn Error),
            Self::External(_) => None,
        }
    }
}

/// Status from `ErrorCode::http_status`; body is the serialized `AppError`
/// so the client can show the original message.
#[cfg(feature = "axum")]
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code().http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

/// A handler's own error passes through; any failure to reach the actor is
/// `Unavailable`, so the server answers 503.
#[cfg(feature = "kameo")]
impl<M> From<SendError<M, AppError>> for AppError {
    fn from(error: SendError<M, AppError>) -> Self {
        match error {
            SendError::HandlerError(error) => error,
            error => crate::err!(Unavailable, "server state is unavailable", @external: error),
        }
    }
}

/// Add context or report once at the chosen boundary, retaining the result.
pub trait ResultExt<T>: Sized {
    fn context(self, message: impl Into<Cow<'static, str>>) -> Result<T>;
    #[track_caller]
    fn error(self) -> Result<T>;
    #[track_caller]
    fn warn(self) -> Result<T>;
    #[track_caller]
    fn debug(self) -> Result<T>;
    #[track_caller]
    fn trace(self) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, message: impl Into<Cow<'static, str>>) -> Result<T> {
        self.map_err(|error| error.context(message))
    }

    #[track_caller]
    fn error(self) -> Result<T> {
        if let Err(error) = &self {
            let caller = Location::caller();
            tracing::error!(error = ?error, caller.file = caller.file(), caller.line = caller.line());
        }
        self
    }

    #[track_caller]
    fn warn(self) -> Result<T> {
        if let Err(error) = &self {
            let caller = Location::caller();
            tracing::warn!(error = ?error, caller.file = caller.file(), caller.line = caller.line());
        }
        self
    }

    #[track_caller]
    fn debug(self) -> Result<T> {
        if let Err(error) = &self {
            let caller = Location::caller();
            tracing::debug!(error = ?error, caller.file = caller.file(), caller.line = caller.line());
        }
        self
    }

    #[track_caller]
    fn trace(self) -> Result<T> {
        if let Err(error) = &self {
            let caller = Location::caller();
            tracing::trace!(error = ?error, caller.file = caller.file(), caller.line = caller.line());
        }
        self
    }
}

// Macro helpers stay under this crate's paths so consumers need no JSON import.
#[doc(hidden)]
pub use serde_json as __serde_json;

#[doc(hidden)]
pub fn __with_data<T: Serialize>(error: AppError, data: T) -> AppError {
    with_serialized_data(error, serde_json::to_value(data))
}

#[doc(hidden)]
pub fn __with_object_data(
    error: AppError,
    entries: impl IntoIterator<Item = (&'static str, serde_json::Result<Value>)>,
) -> AppError {
    let data: serde_json::Result<serde_json::Map<String, Value>> = entries
        .into_iter()
        .map(|(key, value)| value.map(|value| (key.to_owned(), value)))
        .collect();
    with_serialized_data(error, data.map(Value::Object))
}

fn with_serialized_data(mut error: AppError, data: serde_json::Result<Value>) -> AppError {
    let data = data.ok();
    match &mut error {
        AppError::Internal(error) => error.data = data,
        AppError::External(error) => error.data = data,
    }
    error
}

/// Construct a categorized error value with optional data and causes.
///
/// Supports messages, formatting arguments and chained modifiers. Foreign errors
/// become strings; JSON conversion failures omit data rather than failing.
#[macro_export]
macro_rules! err {
    (@apply $err:expr $(,)?) => { $err };

    (@apply $err:expr, @data: { $($key:ident: $val:expr),* $(,)? } $($rest:tt)*) => {
        $crate::err!(@apply $crate::error::__with_object_data(
            $err,
            [$( (::core::stringify!($key), $crate::error::__serde_json::to_value(&$val)) ),*],
        ) $($rest)*)
    };

    (@apply $err:expr, @data: $data:expr $(, $($rest:tt)*)?) => {
        $crate::err!(@apply $crate::error::__with_data($err, $data) $(, $($rest)*)?)
    };

    (@apply $err:expr, @source: $source:expr $(, $($rest:tt)*)?) => {
        $crate::err!(@apply ($err).with_cause($source) $(, $($rest)*)?)
    };

    (@apply $err:expr, @external: $source:expr $(, $($rest:tt)*)?) => {
        $crate::err!(@apply {
            let error = $err;
            let code = *error.code();
            error.with_cause($crate::error::AppError::external(code, $source))
        } $(, $($rest)*)?)
    };

    (@external $code:expr, $source:expr) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::error::AppError::external($code, $source)
    }};

    (@external $code:expr, $source:expr, @data: $data:expr) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::error::__with_data($crate::error::AppError::external($code, $source), $data)
    }};

    ($code:expr, $msg:expr, @$($mods:tt)+) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::err!(@apply $crate::error::AppError::internal($code, $msg), @$($mods)+)
    }};

    ($code:expr, $fmt:expr, $($arg:expr),+, @$($mods:tt)+) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::err!(@apply
            $crate::error::AppError::internal($code, ::std::format!($fmt, $($arg),+)),
            @$($mods)+
        )
    }};

    ($code:expr, $msg:expr) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::error::AppError::internal($code, $msg)
    }};

    ($code:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::error::ErrorCode::*;
        $crate::error::AppError::internal($code, ::std::format!($fmt, $($arg),+))
    }};
}
