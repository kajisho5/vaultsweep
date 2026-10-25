// リリース版では黒い画面（コンソール）を出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vaultsweep_app::run()
}
