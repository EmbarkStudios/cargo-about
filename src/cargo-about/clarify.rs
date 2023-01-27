use anyhow::Context as _;
use cargo_about::licenses::fetch::GitCache;
use krates::Utf8PathBuf as PathBuf;

fn parse_subsection(s: &str) -> anyhow::Result<(Option<String>, Option<String>)> {
    let pos = s
        .find("!!")
        .with_context(|| format!("unable to find '!!' in {s}"))?;

    let start = &s[..pos];
    let end = &s[pos + 1..];

    Ok((
        (!start.is_empty()).then(|| start.to_owned()),
        (!end.is_empty()).then(|| end.to_owned()),
    ))
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Reads the license information from a path on disk
    Path {
        /// The path root
        root: PathBuf,
    },
    /// Pulls the file from a git repository
    Repo {
        /// The git revision to retrieve. Can either be a commit hash or a tag.
        rev: String,
        /// The full URL to the git repo. Only `github.com`, `gitlab.com`, and `bitbucket.org` are currently supported.
        repo: url::Url,
    },
    /// Retrieves the file from the git repository and commit associated with
    /// the specified crate and version
    Crate {
        /// The crate's `<name>-<version>` spec to retrieve. The crate source must already be downloaded.
        spec: String,
    },
}

#[derive(clap::Parser, Debug)]
pub struct Args {
    /// One or more subsections in the file which is itself its own license.
    /// Uses `!!` as the separator between the start and end of the subsection
    #[clap(long, short, value_parser = parse_subsection)]
    subsections: Vec<(Option<String>, Option<String>)>,
    /// The minimum confidence score a license must have
    #[clap(long, default_value = "0.8")]
    threshold: f32,
    /// The relative file path from the root of the source
    path: PathBuf,
    #[clap(subcommand)]
    cmd: Subcommand,
}

pub fn cmd(args: Args) -> anyhow::Result<()> {
    let contents = match args.cmd {
        Subcommand::Path { root } => {
            let full_path = root.join(&args.path);
            std::fs::read_to_string(&full_path)
                .with_context(|| format!("unable to read file '{full_path}'"))?
        }
        Subcommand::Repo { rev, repo } => {
            let gc = GitCache::default();

            gc.retrieve_remote(repo.as_str(), &rev, &args.path)
                .context("failed to retrieve remote file")?
        }
        Subcommand::Crate { spec } => {
            // Just hardcoding to the typical because I can't be bothered
            let root = PathBuf::from_path_buf(
                home::cargo_home()
                    .context("unable to find CARGO_HOME directory")?
                    .join("registry/src/github.com-1ecc6299db9ec823"),
            )
            .map_err(|_e| anyhow::anyhow!("CARGO_HOME directory is not utf-8"))?;

            let crate_path = root.join(spec);

            anyhow::ensure!(crate_path.exists(), "unable to find crate source");

            let manifest = std::fs::read_to_string(crate_path.join("Cargo.toml"))
                .context("failed to read Cargo.toml")?;

            #[derive(serde::Deserialize)]
            struct Pkg {
                repository: String,
            }

            #[derive(serde::Deserialize)]
            struct MinPkg {
                package: Pkg,
            }

            let pkg: MinPkg =
                toml::from_str(&manifest).context("failed to deserialize Cargo.toml")?;

            let gc = GitCache::default();
            let vcs_info = GitCache::parse_vcs_info(&crate_path.join(".cargo_vcs_info.json"))
                .context("failed to read sha1")?;

            gc.retrieve_remote(&pkg.package.repository, &vcs_info.git.sha1, &args.path)
                .context("failed to retrieve remote file")?
        }
    };

    let subsections = if args.subsections.is_empty() {
        vec![(0..contents.len(), (None, None))]
    } else {
        let mut subs = Vec::with_capacity(args.subsections.len());

        for (start, end) in args.subsections {
            let start_ind = match &start {
                Some(start) => contents
                    .find(start)
                    .with_context(|| format!("unable to find start text '{start}'"))?,
                None => 0,
            };

            let end_ind = match &end {
                Some(end) => {
                    contents[start_ind..]
                        .find(end)
                        .with_context(|| format!("unable to find end text '{end}'"))?
                        + start_ind
                        + end.len()
                }
                None => 0,
            };

            subs.push((start_ind..end_ind, (start, end)));
        }

        subs
    };

    if contents.contains('\r') {
        log::warn!("{} contains CRLF line endings, the checksums will be calculated with normal LF line endings to match checksum verification", args.path);
    }

    let license_store = cargo_about::licenses::store_from_cache()?;

    let strategy = askalono::ScanStrategy::new(&license_store)
        .mode(askalono::ScanMode::Elimination)
        .confidence_threshold(((args.threshold * 100.0) as u32).clamp(10, 100) as f32 / 100.0)
        .optimize(false)
        .max_passes(1);

    let mut final_expression = String::new();
    let mut files = Vec::with_capacity(subsections.len());

    use cargo_about::licenses::config::{Clarification, ClarificationFile};

    let file_name = args.path.file_name().unwrap().to_owned();

    for (ind, (subrange, (start, end))) in subsections.into_iter().enumerate() {
        let subsection = &contents[subrange];

        let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);

        ctx.update(subsection.as_bytes());

        // TODO: Warn on carriage returns?
        // for line in subsection.split('\r') {
        //     ctx.update(line.as_bytes());
        // }

        let checksum = ctx.finish();

        let text = askalono::TextData::new(subsection);
        let scan_result = strategy
            .scan(&text)
            .with_context(|| format!("failed to scan subsection for license:\n{subsection}"))?;

        let found_license = scan_result.license.with_context(|| {
            format!("failed to discern license for subsection:\n{subsection}")
        })?;
        let license = spdx::license_id(found_license.name).with_context(|| {
            format!(
                "detected license '{}' which is not a valid SPDX identifier",
                found_license.name
            )
        })?;

        println!("--- {ind}\n{subsection}\n{ind} ---");
        println!(
            "license: {} , confidence: {}",
            license.name, scan_result.score
        );

        // Some license files (read: ring) can have duplicates of the same license
        // but with different copyright holders
        if !final_expression.contains(license.name) {
            if ind > 0 {
                final_expression.push_str(" AND ");
            }

            final_expression.push_str(license.name);
        } else {
            log::info!(
                "ignoring license '{}', already present in expression",
                license.name
            );
        }

        files.push(ClarificationFile {
            path: file_name.clone().into(),
            license: Some(spdx::Expression::parse(license.name).with_context(|| {
                format!("failed to parse license {} as an expression", license.name)
            })?),
            checksum: cargo_about::to_hex(checksum.as_ref()),
            start,
            end,
        });
    }

    let overall_expression = spdx::Expression::parse(&final_expression).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse '{}' as the total expression for all of the licenses: {}",
            final_expression,
            e,
        )
    })?;

    let clarification = Clarification {
        license: overall_expression,
        override_git_commit: None,
        files,
        git: Vec::new(),
    };

    let clar_toml =
        toml::to_string_pretty(&clarification).context("failed to serialize to toml")?;

    println!("{clar_toml}");

    Ok(())
}
