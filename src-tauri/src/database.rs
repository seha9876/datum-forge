//! SQLiteバックエンドのmodule facadeです。
//!
//! `database::*`の公開形状を保ちながら、実装だけを責務別moduleへ分けます。
//! Tauri command名やDBスキーマを変えないことが、この階層の前提です。
mod import_export;
mod models;
mod schema;
mod settings;
mod tables;
mod tags;
mod validation;
mod view_layout;
mod view_nav;

pub use models::*;
