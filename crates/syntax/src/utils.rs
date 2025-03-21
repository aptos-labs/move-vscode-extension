#[macro_export]
macro_rules! unwrap_or_return {
    ($e: expr, $ret: expr) => {
        if let Some(it) = $e {
            it
        } else {
            return $ret;
        }
    };
}

pub use unwrap_or_return;

#[macro_export]
macro_rules! unwrap_or_continue {
    ($e: expr) => {{
        let Some(it) = $e else {
            continue;
        };
        it
    }};
}

pub use unwrap_or_continue;
