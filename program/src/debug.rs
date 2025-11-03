/// Debug macro that wraps pinocchio_log::log!
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "debug")]
        pinocchio_log::log!($($arg)*)
    };
}
