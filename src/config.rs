use std::sync::LazyLock;

/// Declares a lazily-initialized static variable that reads its value from a
/// compile-time environment variable with a `MESON_` prefix.
///
/// Panics on first access if the environment variable was not set during compilation.
macro_rules! config_var {
    ($name:ident) => {
        #[allow(clippy::option_env_unwrap)]
        pub static $name: LazyLock<&'static str> = LazyLock::new(|| {
            option_env!(concat!("MESON_", stringify!($name))).expect(concat!(
                "MESON_",
                stringify!($name),
                " was not set at compile time"
            ))
        });
    };
}

config_var!(APP_ID);
config_var!(GETTEXT_PACKAGE);
config_var!(LOCALEDIR);
config_var!(PKGDATADIR);
config_var!(PROFILE);
config_var!(VERSION);
config_var!(RESOURCES_FILE);
