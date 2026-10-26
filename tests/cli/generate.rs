use crate::utils::*;

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn fails_when_templates_arg_missing() -> Result<()> {
    let package = Package::builder().build()?;

    CargoAbout::new(&package)?
        .generate()
        .assert()
        .failure()
        .stderr(predicate::str::is_match(
            r"handlebars template\(s\) must be specified when using handlebars output format",
        )?);

    Ok(())
}

#[test]
fn fails_when_manifest_absent() -> Result<()> {
    let package = Package::builder().no_manifest().build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .failure()
        .stderr(predicate::str::is_match(
            r"cargo manifest path '.*' does not exist",
        )?);

    Ok(())
}

#[test]
fn fails_when_manifest_invalid() -> Result<()> {
    let package = Package::builder().file("Cargo.toml", "").build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse manifest"));

    Ok(())
}

#[test]
fn fails_back_to_default_about_config_when_absent() -> Result<()> {
    let package = Package::builder().no_about_config().build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .stderr(predicate::str::contains(
            "no 'about.toml' found, falling back to default configuration",
        ));

    Ok(())
}

#[test]
fn fails_when_template_file_missing() -> Result<()> {
    let package = Package::builder().no_template().build()?;

    CargoAbout::new(&package)?
        .generate()
        .template("non-existent-about.hbs")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "template(s) path 'non-existent-about.hbs' does not exist",
        ));

    Ok(())
}

#[test]
fn reports_no_licenses_when_no_licenses() -> Result<()> {
    let package = Package::builder().build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

#[test]
fn fails_when_missing_accepted_field() -> Result<()> {
    let package = Package::builder().file("about.toml", "").build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing field 'accepted'"));

    Ok(())
}

#[test]
fn reports_no_licenses_when_no_license_and_accepted_field_empty() -> Result<()> {
    let package = Package::builder().build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

#[test]
fn fails_when_license_field_valid_and_accepted_field_empty() -> Result<()> {
    let package = Package::builder().license(Some("MIT")).build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "failed to satisfy license requirements",
        ))
        .stdout("");

    Ok(())
}

#[test]
fn reports_no_licenses_when_license_field_unknown() -> Result<()> {
    let package = Package::builder()
        .license(Some("MIT"))
        .accepted(&["MIT"])
        .license(Some("UNKNOWN"))
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "unable to parse license expression for 'package 0.0.0': UNKNOWN",
        ))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

#[test]
fn reports_a_license_when_license_field_valid() -> Result<()> {
    let package = Package::builder()
        .license(Some("MIT"))
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr("")
        .stdout(overview_count(1))
        .stdout(licenses_count(1))
        .stdout(contains_default_mit_license_content());

    Ok(())
}

// TODO: might be nice to let the user know that there was a license file field, but
// that the file was missing.
#[test]
fn reports_no_licenses_when_license_file_field_but_no_file() -> Result<()> {
    let package = Package::builder()
        .license_file("LICENSE", None)
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

// TODO: might be nice to let the user know that there was a license file field, but
// that the file was empty / unrecognizable.
#[test]
fn reports_no_licenses_when_license_file_field_but_file_empty() -> Result<()> {
    let package = Package::builder()
        .license_file("LICENSE", Some(""))
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

// TODO: This seems like incorrect behavior.... IMO the report should be generated
// and maybe custom and/or non-accepted licenses should be included with some
// additional metadata noting that it is not accepted..
#[test]
fn reports_no_licenses_when_manifest_has_license_file_field_with_non_spdx_text() -> Result<()> {
    let package = Package::builder()
        .license_file(
            "LICENSE",
            Some("Copyright (c) 2022 Big Birdz. No permissions granted ever."),
        )
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

#[test]
fn reports_custom_spdx_license_text_when_manifest_has_license_file_field_with_spdx_text()
-> Result<()> {
    let license_text = mit_license_text("2022", "Big Birdz");

    let package = Package::builder()
        .license_file("LICENSE", Some(&license_text))
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        // TODO: There should not be a warning about a missing licenses field
        // since the manifest does have a license file field and according to the
        // cargo docs, a manifest should have a license field or a license file
        // field but not both.
        .stderr(contains_missing_license_field_warning(&package))
        .stdout(overview_count(1))
        .stdout(licenses_count(1))
        .stdout(predicates::str::contains(&license_text));

    Ok(())
}

#[test]
fn reports_no_licenses_when_manifest_has_license_file_field_with_spdx_license_text_and_non_std_filename()
-> Result<()> {
    let license_text = mit_license_text("2022", "Big Birdz");

    let package = Package::builder()
        .license_file("NON_STD_LICENSE_FILENAME", Some(&license_text))
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        // TODO: There should not be a warning about a missing licenses field
        // since the manifest does have a license file field and according to the
        // cargo docs, a manifest should have a license field or a license file
        // field but not both.
        .stderr(contains_missing_license_field_warning(&package))
        // TODO: I would've expected this test case to succeed given that the
        // name of the license file is given in the manifest and it is clear
        // that the license can be detected from its text (which works when
        // the license file is named `LICENSE`).
        // I suppose I can understand if scanning all files in the repo would
        // make the tool annoyingly slow.
        .stderr(unable_to_synthesize_license_expr_warning(&package))
        .stdout(overview_count(0))
        .stdout(licenses_count(0));

    Ok(())
}

#[test]
fn reports_custom_spdx_license_file_when_spdx_license_file_has_std_naming_but_not_specifed_in_manifest()
-> Result<()> {
    let license_content = mit_license_text("2022", "Big Birdz");

    let package = Package::builder()
        .license(Some("MIT"))
        .file("LICENSE", &license_content)
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package)?
        .generate()
        .template(package.template()?)
        .assert()
        .success()
        .stderr("")
        .stdout(overview_count(1))
        .stdout(licenses_count(1))
        .stdout(predicates::str::contains(&license_content));

    Ok(())
}

#[test]
fn reports_one_license_when_when_dependency_has_same_spdx_license_and_text() -> Result<()> {
    let package_b = Package::builder()
        .name("package-b")
        .license(Some("MIT"))
        .build()?;

    let package_a = Package::builder()
        .name("package-a")
        .license(Some("MIT"))
        .accepted(&["MIT"])
        .dependency(&package_b)
        .build()?;

    CargoAbout::new(&package_a)?
        .generate()
        .template(package_a.template()?)
        .assert()
        .success()
        .stderr("")
        .stdout(overview_count(1))
        .stdout(licenses_count(1))
        .stdout(contains_default_mit_license_content());

    Ok(())
}

#[test]
fn reports_all_licenses_when_dependency_has_same_spdx_license_and_different_text() -> Result<()> {
    let package_b_license_text = mit_license_text("2022", "Package B Owner");
    let package_b = Package::builder()
        .license_file("LICENSE", Some(&package_b_license_text))
        .name("package-b")
        .build()?;

    let package_a_license_text = mit_license_text("2022", "Package A Owner");
    let package_a = Package::builder()
        .name("package-a")
        .license_file("LICENSE", Some(&package_a_license_text))
        .dependency(&package_b)
        .accepted(&["MIT"])
        .build()?;

    CargoAbout::new(&package_a)?
        .generate()
        .template(package_a.template()?)
        .assert()
        .success()
        .stdout(overview_count(1))
        .stdout(licenses_count(2))
        .stdout(predicates::str::contains(package_a_license_text))
        .stdout(predicates::str::contains(package_b_license_text));

    Ok(())
}

#[test]
fn reports_all_licenses_when_dependency_has_different_license_and_text() -> Result<()> {
    let package_b = Package::builder()
        .name("package-b")
        .license(Some("Apache-2.0"))
        .build()?;

    let package_a = Package::builder()
        .name("package-a")
        .license(Some("MIT"))
        .dependency(&package_b)
        .accepted(&["MIT", "Apache-2.0"])
        .build()?;

    CargoAbout::new(&package_a)?
        .generate()
        .template(package_a.template()?)
        .assert()
        .success()
        .stdout(overview_count(2))
        .stdout(licenses_count(2))
        .stdout(contains_default_mit_license_content())
        .stdout(contains_default_apache2_license_content());

    Ok(())
}

#[test]
fn fails_when_dependency_has_non_accepted_license_field() -> Result<()> {
    let mut package_builder = Package::builder();

    let package_b = package_builder
        .license(Some("Apache-2.0"))
        .name("package-b")
        .build()?;

    let package_a = package_builder
        .license(Some("MIT"))
        .name("package-a")
        .accepted(&["MIT"])
        .dependency(&package_b)
        .build()?;

    CargoAbout::new(&package_a)?
        .generate()
        .template(package_a.template()?)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "encountered 1 errors resolving licenses, unable to generate output",
        ));

    Ok(())
}

fn generate_json(package: &Package) -> Result<serde_json::Value> {
    let output = CargoAbout::new(package)?
        .generate()
        .arg("--offline")
        .arg("--format=json")
        .assert()
        .success();

    Ok(serde_json::from_slice(&output.get_output().stdout)?)
}

#[test]
fn reports_notices_without_changing_license_groups() -> Result<()> {
    let notice_a = "Package A\r\n  Copyright <Alice> & contributors.\r\n";
    let notice_b = "Package B\nCopyright Bob.\n";
    let package_b = Package::builder()
        .name("package-b")
        .license(Some("Apache-2.0"))
        .file("NOTICE", notice_b)
        .build()?;
    let package_a = Package::builder()
        .name("package-a")
        .license(Some("Apache-2.0"))
        .accepted(&["Apache-2.0"])
        .file("NOTICE", notice_a)
        .dependency(&package_b)
        .build()?;

    let report = generate_json(&package_a)?;
    let licenses = report["licenses"].as_array().unwrap();
    assert_eq!(licenses.len(), 1);
    assert_eq!(licenses[0]["id"], "Apache-2.0");
    assert_eq!(licenses[0]["used_by"].as_array().unwrap().len(), 2);
    assert!(licenses[0]["source_path"].is_null());
    assert_eq!(
        licenses[0]["text"],
        spdx::license_id("Apache-2.0").unwrap().text()
    );
    assert_eq!(report["overview"][0]["count"], 2);

    let notices = report["notices"].as_array().unwrap();
    assert_eq!(notices.len(), 2);
    for (notice, name, text) in [
        (&notices[0], "package-a", notice_a),
        (&notices[1], "package-b", notice_b),
    ] {
        assert_eq!(notice["crate"]["name"], name);
        assert_eq!(notice["crate"]["version"], "0.0.0");
        assert_eq!(notice["text"], text);
        let manifest = std::path::Path::new(notice["crate"]["manifest_path"].as_str().unwrap());
        assert_eq!(
            notice["source_path"],
            serde_json::to_value(manifest.with_file_name("NOTICE"))?
        );
    }

    Ok(())
}

#[test]
fn collects_notice_variants_and_respects_scan_filters() -> Result<()> {
    let package = Package::builder()
        .license(Some("MIT"))
        .file("NOTICE.txt", "Root notice")
        .file("third-party/NOTICE-OTHER", "Nested attribution")
        .file("third-party/NOTICE.md", "Nested markdown notice")
        .file("NOTICES.txt", "Unrelated filename")
        .file(".hidden/NOTICE", "Hidden notice")
        .file("ignored/NOTICE", "Ignored notice")
        .file(".ignore", "ignored/\n")
        .build()?;

    for (config, expected) in [
        (
            "accepted = ['MIT']",
            vec![
                "Root notice",
                "Nested attribution",
                "Nested markdown notice",
            ],
        ),
        ("accepted = ['MIT']\nmax-depth = 1", vec!["Root notice"]),
    ] {
        package.dir.child(ABOUT_CONFIG_FILENAME).write_str(config)?;
        let report = generate_json(&package)?;
        let texts: Vec<_> = report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|notice| notice["text"].as_str().unwrap())
            .collect();
        assert_eq!(texts, expected);
    }

    Ok(())
}

#[test]
fn collects_notices_from_clarified_crates() -> Result<()> {
    let license_text = mit_license_text("2026", "Clarified Owner");
    let checksum: String = ring::digest::digest(&ring::digest::SHA256, license_text.as_bytes())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let config = format!(
        "accepted = ['MIT']\n\
        [package.clarify]\n\
        license = 'MIT'\n\
        [[package.clarify.files]]\n\
        path = 'LICENSE'\n\
        checksum = '{checksum}'\n"
    );
    let package = Package::builder()
        .license(Some("Apache-2.0"))
        .file("LICENSE", &license_text)
        .file("NOTICE", "Clarified crate attribution")
        .file(ABOUT_CONFIG_FILENAME, &config)
        .build()?;

    let report = generate_json(&package)?;
    assert_eq!(report["licenses"][0]["id"], "MIT");
    assert_eq!(report["licenses"][0]["text"], license_text);
    assert_eq!(report["notices"].as_array().unwrap().len(), 1);
    assert_eq!(report["notices"][0]["text"], "Clarified crate attribution");

    Ok(())
}

#[test]
fn excludes_notices_from_ignored_crates() -> Result<()> {
    let private = Package::builder()
        .name("private-crate")
        .file(CARGO_MANIFEST_FILENAME, "[package]\nname = 'private-crate'\nversion = '0.0.0'\nlicense = 'Apache-2.0'\npublish = false\n")
        .file("NOTICE", "Private attribution")
        .build()?;
    let package = Package::builder()
        .license(Some("MIT"))
        .dependency(&private)
        .file("NOTICE", "Public attribution")
        .file(
            ABOUT_CONFIG_FILENAME,
            "accepted = ['MIT']\nprivate = { ignore = true }",
        )
        .build()?;

    let report = generate_json(&package)?;
    assert_eq!(report["notices"].as_array().unwrap().len(), 1);
    assert_eq!(report["notices"][0]["crate"]["name"], "package");
    assert_eq!(report["notices"][0]["text"], "Public attribution");

    Ok(())
}

#[test]
fn does_not_infer_a_license_from_notice_text() -> Result<()> {
    let text = mit_license_text("2026", "Notice Owner");
    let package = Package::builder().file("NOTICE", &text).build()?;

    let report = generate_json(&package)?;
    assert!(report["licenses"].as_array().unwrap().is_empty());
    assert_eq!(report["notices"].as_array().unwrap().len(), 1);
    assert_eq!(report["notices"][0]["text"], text);

    Ok(())
}

#[test]
fn renders_notices_in_bundled_templates_only_when_present() -> Result<()> {
    for has_notice in [true, false] {
        let mut builder = Package::builder();
        builder.license(Some("MIT")).accepted(&["MIT"]);
        if has_notice {
            builder.file("NOTICE", "Copyright <Alice> & contributors.\n");
        }
        let package = builder.build()?;

        for template in [
            concat!(env!("CARGO_MANIFEST_DIR"), "/resources/default.hbs"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/about.hbs"),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/about_list_by_crate_example.hbs"
            ),
        ] {
            let output = CargoAbout::new(&package)?
                .generate()
                .arg("--offline")
                .template(template)
                .assert()
                .success()
                .stderr("");

            if has_notice {
                output
                    .stdout(predicate::str::contains("<h2>Notices</h2>"))
                    .stdout(predicate::str::contains("<h3>package 0.0.0</h3>"))
                    .stdout(predicate::str::contains("<pre class=\"license-text\">Copyright &lt;Alice&gt; &amp; contributors.\n</pre>"));
            } else {
                output.stdout(predicate::str::contains("<h2>Notices</h2>").not());
            }
        }

        if !has_notice {
            assert!(
                generate_json(&package)?["notices"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }

    Ok(())
}
