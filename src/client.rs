use anyhow::{bail, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{fmt::Display, time::Duration};

pub struct Requester {
    client: Client,
}

impl Requester {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(9))
                .user_agent(format!("tropicbliss/caulk/{}", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }

    pub fn get_latest_minecraft_version(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Output {
            version: String,
            version_type: String,
        }

        Ok(self
            .client
            .get("https://api.modrinth.com/v2/tag/game_version")
            .send()?
            .json::<Vec<Output>>()?
            .into_iter()
            .find(|version| version.version_type == "release")
            .unwrap()
            .version)
    }

    pub fn get_queries(
        &self,
        query: &str,
        target_version: &str,
        loader: &str,
    ) -> Result<Vec<Project>> {
        #[derive(Deserialize)]
        struct Output {
            hits: Vec<Project>,
        }

        let res = self
            .client
            .get("https://api.modrinth.com/v2/search")
            .query(&[("query", query)])
            .query(&[(
                "facets",
                format!(r#"[["versions:{target_version}"], ["categories:{loader}"]]"#),
            )])
            .send()?
            .json::<Output>()?;
        Ok(res.hits)
    }

    pub fn get_download_url(&self, id: &str, target_version: &str, loader: &str) -> Result<Link> {
        #[derive(Deserialize)]
        struct Output {
            game_versions: Vec<String>,
            loaders: Vec<String>,
            files: Vec<OutputFile>,
            dependencies: Option<Vec<DependencyOutput>>,
        }

        #[derive(Deserialize)]
        struct OutputFile {
            url: String,
            filename: String,
        }

        #[derive(Deserialize)]
        struct DependencyOutput {
            pub project_id: Option<String>,
            pub dependency_type: String,
        }

        let res = self
            .client
            .get(format!("https://api.modrinth.com/v2/project/{id}/version"))
            .send()?
            .json::<Vec<Output>>()?;
        let version = res.into_iter().find_map(|v| {
            if v.game_versions.contains(&target_version.to_string())
                && v.loaders.contains(&loader.to_string())
            {
                let file = v.files.into_iter().find(|f| f.filename.ends_with(".jar"));
                if let Some(file) = file {
                    return Some(Link {
                        url: file.url,
                        filename: file.filename,
                        dependencies: v
                            .dependencies
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|dep| {
                                if let Some(proj_id) = dep.project_id {
                                    Some(Dependency {
                                        dependency_type: dep.dependency_type,
                                        project_id: proj_id,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    });
                } else {
                    return None;
                }
            }
            None
        });
        if let Some(version) = version {
            Ok(version)
        } else {
            bail!("No jar files found");
        }
    }

    pub fn download_file(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self.client.get(url).send()?.bytes()?.into())
    }

    pub fn get_project_name(&self, id: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Output {
            title: String,
        }

        Ok(self
            .client
            .get(format!("https://api.modrinth.com/v2/project/{id}"))
            .send()?
            .json::<Output>()?
            .title)
    }
}

pub struct Link {
    pub url: String,
    pub filename: String,
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
pub struct Dependency {
    pub project_id: String,
    pub dependency_type: String,
}

#[derive(Deserialize)]
pub struct Project {
    pub project_id: String,
    pub title: String,
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}
