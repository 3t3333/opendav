use std::fs;
use std::path::{Path, PathBuf};

use super::repository::{RepositoryError, SimGitRepository};

#[derive(Debug, Clone)]
pub struct SimGitManager {
    pub root_dir: PathBuf,
    pub active_project: Option<String>,
}

impl SimGitManager {
    pub fn new(root_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root_dir);
        let active_project = fs::read_to_string(root_dir.join(".active_repository"))
            .ok()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty() && root_dir.join(name).is_dir());
        Self {
            root_dir,
            active_project,
        }
    }

    pub fn set_active_project(&mut self, project_name: &str) -> Result<(), RepositoryError> {
        let repository = SimGitRepository::open(&self.root_dir, project_name)?;
        fs::write(
            self.root_dir.join(".active_repository"),
            repository.name().as_bytes(),
        )?;
        self.active_project = Some(repository.name().to_owned());
        Ok(())
    }

    pub fn create_project(&mut self, project_name: &str) -> Result<(), RepositoryError> {
        let repository = SimGitRepository::create(&self.root_dir, project_name)?;
        self.set_active_project(repository.name())
    }

    pub fn delete_project(&mut self, project_name: &str) -> Result<(), RepositoryError> {
        let repo_path = self.root_dir.join(project_name);
        if repo_path.is_dir() {
            fs::remove_dir_all(&repo_path).map_err(RepositoryError::Io)?;
        }
        if self.active_project.as_deref() == Some(project_name) {
            self.active_project = None;
            let _ = fs::remove_file(self.root_dir.join(".active_repository"));
        }
        Ok(())
    }

    pub fn active_repository(&self) -> Result<SimGitRepository, RepositoryError> {
        let name = self
            .active_project
            .as_deref()
            .ok_or(RepositoryError::InvalidName)?;
        SimGitRepository::open(&self.root_dir, name)
    }

    pub fn repository(&self, project_name: &str) -> Result<SimGitRepository, RepositoryError> {
        SimGitRepository::open(&self.root_dir, project_name)
    }

    pub fn list_projects(&self) -> Vec<String> {
        let mut projects = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
                    if let Some(name) = entry.file_name().to_str() {
                        projects.push(name.to_owned());
                    }
                }
            }
        }
        projects.sort_unstable();
        projects
    }

    pub fn root(&self) -> &Path {
        &self.root_dir
    }
}
