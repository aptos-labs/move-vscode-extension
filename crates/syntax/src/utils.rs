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
