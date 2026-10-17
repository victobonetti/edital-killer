//! SQLite, migrações e repositórios.

pub mod analise_repo;
pub mod config_repo;
pub mod db;
pub mod edital_repo;
pub mod job_repo;
pub mod paths;

pub use db::Db;
pub use paths::AppPaths;
