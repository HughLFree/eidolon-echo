#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Desktop shell binary entrypoint: delegates startup to desktop_pet module.

mod desktop_pet;
mod overlay;

fn main() {
    desktop_pet::run();
}
