/// Macro for conditional debug output (only prints in debug builds)
/// Private to crate, available everywhere via #[macro_use]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!($($arg)*);
    };
}