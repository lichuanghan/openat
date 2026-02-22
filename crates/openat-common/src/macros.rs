//! Common macros

/// Macro to create a quick error
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        anyhow::anyhow!($($arg)*)
    };
}

/// Macro to bail with a quick error
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err(anyhow::anyhow!($($arg)*))
    };
}

/// Macro to unwrap with context
#[macro_export]
macro_rules! context {
    ($expr:expr, $($arg:tt)*) => {
        $expr.context($($arg)*)
    };
}

/// Macro to log and continue on error
#[macro_export]
macro_rules! log_error {
    ($expr:expr) => {
        match $expr {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!("Error: {}", e);
                None
            }
        }
    };
    ($expr:expr, $($arg:tt)*) => {
        match $expr {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!($($arg)*, error = %e);
                None
            }
        }
    };
}
