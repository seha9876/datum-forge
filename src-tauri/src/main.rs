//! Tauri デスクトップアプリのネイティブエントリポイントです。
//!
//! 実際のアプリ初期化処理は `datum_forge_lib::run()` にあります。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    datum_forge_lib::run();
}
