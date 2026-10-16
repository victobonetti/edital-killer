use std::path::{Path, PathBuf};

/// Layout do diretório de dados do app.
#[derive(Debug, Clone)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn new(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let p = Self { root: root.into() };
        std::fs::create_dir_all(p.editais_dir())?;
        std::fs::create_dir_all(p.jobs_dir())?;
        std::fs::create_dir_all(p.logs_dir())?;
        Ok(p)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db_file(&self) -> PathBuf {
        self.root.join("edital-killer.db")
    }

    pub fn editais_dir(&self) -> PathBuf {
        self.root.join("editais")
    }

    pub fn edital_dir(&self, edital_id: &str) -> PathBuf {
        self.editais_dir().join(edital_id)
    }

    pub fn edital_pdf(&self, edital_id: &str) -> PathBuf {
        self.edital_dir(edital_id).join("original.pdf")
    }

    pub fn jobs_dir(&self) -> PathBuf {
        self.root.join("jobs")
    }

    pub fn job_dir(&self, job_id: &str) -> PathBuf {
        self.jobs_dir().join(job_id)
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
}
