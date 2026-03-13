#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Desktop shell binary entrypoint: delegates startup to desktop_pet module.

mod desktop_pet;
mod overlay;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "desktop_ai_shell=info".into()),
        )
        .init();
    desktop_pet::run();
}
