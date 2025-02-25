use super::{Krate, config};
use anyhow::Context as _;
use krates::Utf8Path as Path;
use reqwest::blocking::Client;
use std::{io::Read, sync::Arc};
use url::Url;

#[derive(Copy, Clone, Debug)]
enum GitHostFlavor {
    Github,
    Gitlab,
    Bitbucket,
    // gitea...etc. ugh, this should be standardized somehow
}

impl GitHostFlavor {
    fn from_repo(repo: &Url) -> anyhow::Result<Self> {
        Ok(match repo.domain() {
            Some("github.com") => Self::Github,
            Some("gitlab.com") => Self::Gitlab,
            Some("bitbucket.org") => Self::Bitbucket,
            Some(unsupported) => {
                anyhow::bail!("the git host '{unsupported}' is not supported at this time")
            }
            None => anyhow::bail!("the repo url is malformed and does not contain a domain"),
        })
    }

    /// Fetches the file contents of a path from the specific repository via
    /// a third party site for now until I can find a better solution, that still
    /// doesn't mean requiring access tokens or cloning the entire repository
    fn fetch(self, client: &Client, repo: &Url, rev: &str, path: &Path) -> anyhow::Result<String> {
        let project = repo
            .path()
            .strip_prefix('/')
            .context("repo url does not have valid path")?;

        // Some crates in repos with a workspace will try and be nice and give
        // a subpath as the repo, which is friendly to users, but screws up
        // things here, so we just chop off excess path parameters
        let first = project.find('/').context("expected an <org/repo> path")?;

        let project = match project[first + 1..].find('/') {
            Some(second) => &project[..first + second + 1],
            None => project,
        };

        let req = match self {
            Self::Github => {
                // https://docs.github.com/en/rest/reference/repos#contents
                client.get(format!("https://rawcdn.githack.com/{project}/{rev}/{path}"))
            }
            Self::Gitlab => {
                // https://docs.gitlab.com/ee/api/repository_files.html#get-raw-file-from-repository
                // https://glcdn.githack.com/veloren/veloren/-/raw/f92c6fbd49269b6e2cad04ae229d3405a6656053/LICENSE
                client.get(format!(
                    "https://glcdn.githack.com/{project}/-/raw/{rev}/{path}"
                ))
            }
            Self::Bitbucket => {
                // https://developer.atlassian.com/bitbucket/api/2/reference/resource/repositories/%7Bworkspace%7D/%7Brepo_slug%7D/src/%7Bcommit%7D/%7Bpath%7D
                // https://bbcdn.githack.com/atlassian/pipelines-examples-rust/raw/581100fe400cd0cfb17f54c2aa26121181f82646/README.md
                client.get(format!(
                    "https://bbcdn.githack.com/{project}/raw/{rev}/{path}"
                ))
            }
        };

        let mut res = req
            .send()
            .context("failed to send request")?
            .error_for_status()?;

        let mut contents = String::with_capacity(res.content_length().unwrap_or(1024) as usize);
        res.read_to_string(&mut contents)
            .context("failed to read contents as utf-8")?;

        Ok(contents)
    }
}

/// The information for the git commit when a crate was published
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitInfo {
    pub sha1: String,
}

/// The structure of a `.cargo_vs_info` file
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VcsInfo {
    pub git: GitInfo,
    pub path_in_vcs: Option<krates::Utf8PathBuf>,
}

/// Since it's often the case that the reason a license file is in source control
/// but not in the actual published package is due to it being in the root but
/// not copied into each sub-crate in the repository, we can just not re-retrieve
/// the same file multiple times
#[derive(Clone)]
pub struct GitCache {
    cache: Arc<parking_lot::RwLock<std::collections::HashMap<u64, Arc<String>>>>,
    http_client: Option<Client>,
}

impl GitCache {
    pub fn maybe_offline(http_client: Option<Client>) -> Self {
        Self {
            http_client,
            cache: Default::default(),
        }
    }

    pub fn online() -> Self {
        Self {
            http_client: Some(Client::new()),
            cache: Default::default(),
        }
    }

    #[allow(clippy::unused_self)]
    fn retrieve_local(
        &self,
        krate: &Krate,
        file: &config::ClarificationFile,
    ) -> anyhow::Result<String> {
        // The only reason this kind of clarification should be used is when the
        // file in question is not part of the published package, which is
        // almost always because it is in a parent directory, so we request the
        // location of the workspace root for the manifest with that assumption
        // in mind, though this might fail in more complicated scenarios like if
        // there are multiple workspaces in a single repository
        let mut cmd = std::process::Command::new("cargo");
        cmd.args([
            "locate-project",
            "--workspace",
            "--manifest-path",
            krate.manifest_path.as_str(),
        ]);

        let root = cmd
            .output()
            .context("failed to invoke cargo")
            .and_then(|output| {
                anyhow::ensure!(
                    output.status.success(),
                    "cargo locate-project failed with exit code {}",
                    output.status.code().unwrap_or(-1)
                );

                #[derive(serde::Deserialize)]
                struct Locate {
                    root: super::PathBuf,
                }

                let loc: Locate = serde_json::from_slice(&output.stdout)
                    .context("failed to deserialize locate-project output")?;
                Ok(loc.root)
            })
            .with_context(|| {
                format!("failed to locate workspace root for path dependency '{krate}'")
            })?;

        let license_path = root.parent().unwrap().join(&file.path);

        let contents = std::fs::read_to_string(&license_path)
            .with_context(|| format!("unable to read path '{license_path}'"))?;
        Ok(contents)
    }

    pub fn retrieve_remote(&self, repo: &str, rev: &str, path: &Path) -> anyhow::Result<String> {
        let repo_url = url::Url::parse(repo)
            .with_context(|| format!("unable to parse repository url '{repo}'"))?;

        let http_client = self
            .http_client
            .as_ref()
            .context("unable to fetch remote repository data in offline mode")?;

        // Unfortunately the HTTP retrieval methods for most of the popular
        // providers require an API token to use, so instead we just use a
        // third party CDN, `raw.githack.com` for now until I can find a better
        // solution, but this does limit us severely in the amount of git repo
        // hosts we can support at the moment. I consider this fine for now
        // though, as this is only used as a fallback when a crate is not
        // packaged properly with the license(s) included
        let flavor = GitHostFlavor::from_repo(&repo_url)?;

        flavor
            .fetch(http_client, &repo_url, rev, path)
            .with_context(|| format!("failed to fetch contents of '{path}' from repo '{repo}'"))
    }

    /// Parses a `.cargo_vcs_info.json` located in the root of a packaged crate
    /// and returns the sha1 commit the package was built from
    pub fn parse_vcs_info(vcs_info_path: &Path) -> anyhow::Result<VcsInfo> {
        let vcs_info = std::fs::read_to_string(vcs_info_path)
            .with_context(|| format!("unable to read '{vcs_info_path}'"))?;

        let vcs_info: VcsInfo = serde_json::from_str(&vcs_info)
            .with_context(|| format!("failed to deserialize '{vcs_info_path}'"))?;

        Ok(vcs_info)
    }

    pub(crate) fn retrieve(
        &self,
        krate: &Krate,
        file: &config::ClarificationFile,
        commit_override: &Option<String>,
    ) -> anyhow::Result<Arc<String>> {
        match &krate.source {
            Some(src) => {
                // If we have a git dependency we already have the proper source
                // locally so we don't need to do a remote fetch, however for
                // registry sources, we have the packaged source only, which
                // may not include the file we are looking for, so we need to
                // fetch it with a remote call
                if src.repr.starts_with("git+") {
                    self.retrieve_local(krate, file).map(Arc::new)
                } else if src.repr.starts_with("registry+") {
                    let repo = krate.repository.as_deref().with_context(|| {
                        format!("crate '{krate}' with registry source does not have a 'repository'")
                    })?;

                    let sha1 = if let Some(co) = commit_override {
                        log::debug!("using commit override '{co}' for crate '{krate}'");
                        co.clone()
                    } else {
                        let vcs_info_path = krate
                            .manifest_path
                            .parent()
                            .unwrap()
                            .join(".cargo_vcs_info.json");

                        Self::parse_vcs_info(&vcs_info_path)?.git.sha1
                    };

                    let hash = {
                        use std::hash::Hasher;
                        let mut hasher = twox_hash::XxHash64::default();

                        hasher.write(repo.as_bytes());
                        hasher.write(sha1.as_bytes());
                        hasher.write(file.path.as_str().as_bytes());

                        hasher.finish()
                    };

                    if let Some(text) = self.cache.read().get(&hash) {
                        return Ok(text.clone());
                    }

                    let contents = Arc::new(self.retrieve_remote(repo, &sha1, &file.path)?);

                    self.cache.write().insert(hash, contents.clone());

                    Ok(contents)
                } else {
                    anyhow::bail!("unknown package source '{}' for crate '{krate}'", src.repr);
                }
            }
            None => {
                // No source means this is a path dependency, so we just treat it
                // as a regular path from the crate root
                self.retrieve_local(krate, file).map(Arc::new)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore = "online"]
    fn fetches_github() {
        let contents = GitHostFlavor::Github
            .fetch(
                &Client::new(),
                &Url::parse("https://github.com/EmbarkStudios/cargo-about").unwrap(),
                "6f0d247ee7f7b6842abc180c2e4e96581e454ca8", /* 0.3.0 commit */
                Path::new("LICENSE-MIT"),
            )
            .unwrap();

        crate::validate_sha256(
            &contents,
            "090a294a492ab2f41388252312a65cf2f0e423330b721a68c6665ac64766753b",
        )
        .unwrap();
    }

    #[test]
    #[ignore = "online"]
    fn fetches_gitlab() {
        let contents = GitHostFlavor::Gitlab
            .fetch(
                &Client::new(),
                &Url::parse("https://gitlab.com/veloren/veloren").unwrap(),
                "f92c6fbd49269b6e2cad04ae229d3405a6656053",
                Path::new("LICENSE"),
            )
            .unwrap();

        crate::validate_sha256(
            &contents,
            "38987784a1d1bbf21a8f2401b6cf636addc82071ea4919da9833b1fee48cfd3f",
        )
        .unwrap();
    }

    #[test]
    #[ignore = "online"]
    fn fetches_bitbucket() {
        let contents = GitHostFlavor::Bitbucket
            .fetch(
                &Client::new(),
                &Url::parse("https://bitbucket.org/atlassian/pipelines-examples-rust/").unwrap(),
                "581100fe400cd0cfb17f54c2aa26121181f82646",
                Path::new("README.md"),
            )
            .unwrap();

        crate::validate_sha256(
            &contents,
            "65fa772cee7a8aa36c86444058a65eb7a51ea335030b0c55f3d50a34215d68b4",
        )
        .unwrap();
    }
}
