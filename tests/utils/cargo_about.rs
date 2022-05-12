use crate::utils::Package;

use anyhow::Result;
use assert_cmd::assert::Assert;
use assert_cmd::prelude::*;
use std::process::Command;

pub struct CargoAbout {
    cmd: Command,
}

impl CargoAbout {
    pub fn new(package: &Package) -> Result<Self> {
        let mut cmd = Command::cargo_bin("cargo-about")?;
        cmd.current_dir(&package.dir);
        Ok(CargoAbout { cmd })
    }

    pub fn arg(&mut self, arg: &str) -> &mut Self {
        self.cmd.arg(arg);
        self
    }

    pub fn init(&mut self) -> &mut Self {
        self.arg("init")
    }

    pub fn generate(&mut self) -> &mut Self {
        self.arg("generate")
    }

    pub fn template(&mut self, template: &str) -> &mut Self {
        self.arg(template)
    }

    pub fn assert(&mut self) -> Assert {
        self.cmd.assert()
    }
}
