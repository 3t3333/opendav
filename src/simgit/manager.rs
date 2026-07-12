use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct SimGitManager {
    pub root_dir: PathBuf,
    pub active_project: Option<String>,
}

impl SimGitManager {
    pub fn new(root_dir: PathBuf) -> Self {
        if !root_dir.exists() {
            let _ = fs::create_dir_all(&root_dir);
        }
        Self {
            root_dir,
            active_project: None,
        }
    }

    pub fn set_active_project(&mut self, project_name: &str) {
        self.active_project = Some(project_name.to_string());
    }

    pub fn create_project(&self, project_name: &str) -> Result<(), std::io::Error> {
        let proj_dir = self.root_dir.join(project_name);
        fs::create_dir_all(&proj_dir)?;
        fs::create_dir_all(proj_dir.join("telemetry"))?;
        fs::create_dir_all(proj_dir.join("setups"))?;
        fs::create_dir_all(proj_dir.join("lapfiles"))?;
        fs::create_dir_all(proj_dir.join("exports"))?;
        fs::create_dir_all(proj_dir.join("reports"))?;
        Ok(())
    }

    pub fn list_projects(&self) -> Vec<String> {
        let mut projects = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            projects.push(name.to_string());
                        }
                    }
                }
            }
        }
        projects.sort();
        projects
    }

    pub fn list_setups(&self) -> Vec<PathBuf> {
        let mut setups = Vec::new();
        if let Some(ref proj) = self.active_project {
            let setups_dir = self.root_dir.join(proj).join("setups");
            if let Ok(entries) = fs::read_dir(&setups_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            setups.push(entry.path());
                        }
                    }
                }
            }
        }
        setups.sort();
        setups
    }
}
