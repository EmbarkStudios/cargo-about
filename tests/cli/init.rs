use crate::utils::*;

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn fails_when_manifest_absent() -> Result<()> {
    let package = Package::builder().no_manifest().build()?;

    CargoAbout::new(&package)?
        .init()
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not find `Cargo.toml`"));
    Ok(())
}

#[test]
fn fails_when_manifest_empty() -> Result<()> {
    let package = Package::builder().file("Cargo.toml", "").build()?;

    CargoAbout::new(&package)?
        .init()
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse manifest"));

    Ok(())
}

#[test]
fn writes_config_and_template_by_default() -> Result<()> {
    let package = Package::builder().no_template().no_about_config().build()?;

    CargoAbout::new(&package)?.init().assert().success();

    let dir = &package.dir;
    dir.child(ABOUT_CONFIG_FILENAME)
        .assert(predicate::path::exists());
    dir.child(ABOUT_TEMPLATE_FILENAME)
        .assert(predicate::path::exists());

    Ok(())
}

#[test]
fn writes_config_only_when_no_handlebars_specifed() -> Result<()> {
    let package = Package::builder().no_template().no_about_config().build()?;

    CargoAbout::new(&package)?
        .init()
        .arg("--no-handlebars")
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let dir = &package.dir;
    dir.child(ABOUT_CONFIG_FILENAME)
        .assert(predicate::path::exists());
    dir.child(ABOUT_TEMPLATE_FILENAME)
        .assert(predicate::path::missing());

    Ok(())
}

#[test]
fn does_not_overwrite_by_default() -> Result<()> {
    let template_content = "A useless custom template";
    let config_content = "A useless invalid config";

    let package = Package::builder()
        .file(ABOUT_TEMPLATE_FILENAME, template_content)
        .file(ABOUT_CONFIG_FILENAME, config_content)
        .build()?;

    CargoAbout::new(&package)?
        .init()
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let config = &package.dir.child(ABOUT_CONFIG_FILENAME);
    let template = &package.dir.child(ABOUT_TEMPLATE_FILENAME);

    assert_eq!(std::fs::read_to_string(&config)?, config_content);
    assert_eq!(std::fs::read_to_string(&template)?, template_content);

    Ok(())
}

#[test]
fn overwrites_config_and_template_when_overwrite_specified() -> Result<()> {
    let template_content = "A useless custom template";
    let config_content = "A useless invalid config";

    let package = Package::builder()
        .file(ABOUT_TEMPLATE_FILENAME, template_content)
        .file(ABOUT_CONFIG_FILENAME, config_content)
        .build()?;

    CargoAbout::new(&package)?
        .init()
        .arg("--overwrite")
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let config = &package.dir.child(ABOUT_CONFIG_FILENAME);
    let template = &package.dir.child(ABOUT_TEMPLATE_FILENAME);

    assert_ne!(std::fs::read_to_string(&config)?, config_content);
    assert_ne!(std::fs::read_to_string(&template)?, template_content);

    Ok(())
}

#[test]
fn overwrites_config_only_when_no_handlebars_and_overwrite_specified() -> Result<()> {
    let template_content = "A useless custom template";
    let config_content = "A useless invalid config";

    let package = Package::builder()
        .file(ABOUT_TEMPLATE_FILENAME, template_content)
        .file(ABOUT_CONFIG_FILENAME, config_content)
        .build()?;

    CargoAbout::new(&package)?
        .init()
        .arg("--no-handlebars")
        .arg("--overwrite")
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let config = &package.dir.child(ABOUT_CONFIG_FILENAME);
    let template = &package.dir.child(ABOUT_TEMPLATE_FILENAME);

    assert_ne!(std::fs::read_to_string(&config)?, config_content);
    assert_eq!(std::fs::read_to_string(&template)?, template_content);

    Ok(())
}
