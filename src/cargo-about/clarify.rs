use anyhow::Context as _;
use krates::Utf8PathBuf as PathBuf;
use structopt::StructOpt;

fn parse_subsection(s: &str) -> anyhow::Result<(Option<String>, Option<String>)> {
    let pos = s
        .find("!")
        .with_context(|| format!("unable to find '!' in {}", s))?;

    let start = &s[..pos];
    let end = &s[pos + 1..];

    Ok((
        (!start.is_empty()).then(|| start.to_owned()),
        (!end.is_empty()).then(|| end.to_owned()),
    ))
}

#[derive(StructOpt, Debug)]
pub struct Args {
    /// One or more subsections in the file which is itself its own license
    #[structopt(long, short, parse(try_from_str = parse_subsection))]
    subsections: Vec<(Option<String>, Option<String>)>,
    /// The minimum confidence score a license must have
    #[structopt(long, default_value = "0.8")]
    threshold: f32,
    /// The path of the file to clarify
    path: PathBuf,
}

pub fn cmd(args: Args) -> anyhow::Result<()> {
    let contents = std::fs::read_to_string(&args.path)
        .with_context(|| format!("unable to read file '{}'", args.path))?;

    let subsections = if args.subsections.is_empty() {
        vec![(0..contents.len(), (None, None))]
    } else {
        let mut subs = Vec::with_capacity(args.subsections.len());

        for (start, end) in args.subsections {
            let start_ind = match &start {
                Some(start) => contents
                    .find(start)
                    .with_context(|| format!("unable to find start text '{}'", start))?,
                None => 0,
            };

            let end_ind = match &end {
                Some(end) => {
                    contents[start_ind..]
                        .find(end)
                        .with_context(|| format!("unable to find end text '{}'", end))?
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
        .confidence_threshold(
            std::cmp::max(10, std::cmp::min(100, (args.threshold * 100.0) as u32)) as f32 / 100.0,
        )
        .optimize(false)
        .max_passes(1);

    let mut final_expression = String::new();
    let mut files = Vec::with_capacity(subsections.len());

    use cargo_about::licenses::config::{Clarification, ClarificationFile};

    let file_name = args.path.file_name().unwrap().to_owned();

    for (ind, (subrange, (start, end))) in subsections.into_iter().enumerate() {
        let subsection = &contents[subrange];

        let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);

        for line in subsection.split('\r') {
            ctx.update(line.as_bytes());
        }

        let checksum = ctx.finish();

        let text = askalono::TextData::new(subsection);
        let scan_result = strategy
            .scan(&text)
            .map_err(|e| e.compat())
            .with_context(|| format!("failed to scan subsection for license:\n{}", subsection))?;

        let found_license = scan_result.license.with_context(|| {
            format!("failed to discern license for subsection:\n{}", subsection)
        })?;
        let license = spdx::license_id(&found_license.name).with_context(|| {
            format!(
                "detected license '{}' which is not a valid SPDX identifier",
                found_license.name
            )
        })?;

        println!("--- {0}\n{1}\n{0} ---", ind, subsection);
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
        files,
    };

    let clar_toml =
        toml::to_string_pretty(&clarification).context("failed to serialize to toml")?;

    println!("{}", clar_toml);

    Ok(())
}
