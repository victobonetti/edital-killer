//! Tipos de domínio, sem IO.

pub mod edital;
pub mod extracao;
pub mod job;
pub mod normalize;
pub mod qualidade;
pub mod settings;
pub mod uso;

pub use edital::*;
pub use extracao::*;
pub use job::*;
pub use qualidade::{OrigemTexto, QualidadeTexto};
pub use settings::*;
pub use uso::*;
