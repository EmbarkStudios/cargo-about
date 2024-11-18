#![doc = include_str!("../README.md")]

use krates::cm;
use std::{cmp, fmt};

pub mod licenses;

pub struct Krate(pub cm::Package);

impl Krate {
    fn get_license_expression(&self) -> licenses::LicenseInfo {
        if let Some(license_field) = &self.0.license {
            //. Reasons this can fail:
            // * Empty! The rust crate used to validate this field has a bug
            // https://github.com/rust-lang-nursery/license-exprs/issues/23
            // * It also just does basic lexing, so parens, duplicate operators,
            // unpaired exceptions etc can all fail validation

            match spdx::Expression::parse(license_field) {
                Ok(validated) => licenses::LicenseInfo::Expr(validated),
                Err(err) => {
                    log::error!("unable to parse license expression for '{self}': {err}");
                    licenses::LicenseInfo::Unknown
                }
            }
        } else {
            log::warn!("crate '{self}' doesn't have a license field");
            licenses::LicenseInfo::Unknown
        }
    }
}

impl Ord for Krate {
    #[inline]
    fn cmp(&self, o: &Self) -> cmp::Ordering {
        match self.0.name.cmp(&o.0.name) {
            cmp::Ordering::Equal => self.0.version.cmp(&o.0.version),
            o => o,
        }
    }
}

impl PartialOrd for Krate {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl PartialEq for Krate {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == cmp::Ordering::Equal
    }
}

impl Eq for Krate {}

impl From<cm::Package> for Krate {
    fn from(mut pkg: cm::Package) -> Self {
        // Fix the license field as cargo used to allow the
        // invalid / separator
        if let Some(lf) = &mut pkg.license {
            *lf = lf.replace('/', " OR ");
        }

        Self(pkg)
    }
}

impl krates::KrateDetails for Krate {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn version(&self) -> &krates::semver::Version {
        &self.0.version
    }
}

impl fmt::Display for Krate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0.name, self.0.version)
    }
}

impl std::ops::Deref for Krate {
    type Target = cm::Package;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Krates = krates::Krates<Krate>;

pub fn get_all_crates(
    cargo_toml: &krates::Utf8Path,
    no_default_features: bool,
    all_features: bool,
    features: Vec<String>,
    workspace: bool,
    lock_opts: krates::LockOptions,
    cfg: &licenses::config::Config,
) -> anyhow::Result<Krates> {
    let mut mdc = krates::Cmd::new();
    mdc.manifest_path(cargo_toml);
    mdc.lock_opts(lock_opts);

    // The metadata command builder is weird and only allows you to specify
    // one of these, but really you might need to do multiple of them
    if no_default_features {
        mdc.no_default_features();
    }

    if all_features {
        mdc.all_features();
    }

    mdc.features(features);

    let mut builder = krates::Builder::new();

    if workspace {
        builder.workspace(true);
    }

    if cfg.ignore_build_dependencies {
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    }

    if cfg.ignore_dev_dependencies {
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    }

    if cfg.ignore_transitive_dependencies {
        builder.ignore_kind(krates::DepKind::Normal, krates::Scope::NonWorkspace);
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);
    }

    builder.include_targets(cfg.targets.iter().map(|triple| (triple.as_str(), vec![])));

    let graph = builder.build(mdc, |filtered: cm::Package| {
        if let Some(src) = filtered.source {
            if src.is_crates_io() {
                log::debug!("filtered {} {}", filtered.name, filtered.version);
            } else {
                log::debug!("filtered {} {} {}", filtered.name, filtered.version, src);
            }
        } else {
            log::debug!("filtered crate {} {}", filtered.name, filtered.version);
        }
    })?;

    Ok(graph)
}

#[inline]
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    const CHARS: &[u8] = b"0123456789abcdef";

    for &byte in bytes {
        s.push(CHARS[(byte >> 4) as usize] as char);
        s.push(CHARS[(byte & 0xf) as usize] as char);
    }

    s
}

pub fn validate_sha256(buffer: &str, expected: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        expected.len() == 64,
        "checksum '{expected}' length is {} instead of expected 64",
        expected.len()
    );

    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);

    ctx.update(buffer.as_bytes());

    // Ignore faulty CRLF style newlines
    // for line in buffer.split('\r') {
    //     ctx.update(line.as_bytes());
    // }

    let content_digest = ctx.finish();
    let digest = content_digest.as_ref();

    for (ind, exp) in expected.as_bytes().chunks(2).enumerate() {
        let mut cur = match exp[0] {
            b'A'..=b'F' => exp[0] - b'A' + 10,
            b'a'..=b'f' => exp[0] - b'a' + 10,
            b'0'..=b'9' => exp[0] - b'0',
            c => {
                anyhow::bail!("invalid byte in checksum '{expected}' @ {ind}: {c}");
            }
        };

        cur <<= 4;

        cur |= match exp[1] {
            b'A'..=b'F' => exp[1] - b'A' + 10,
            b'a'..=b'f' => exp[1] - b'a' + 10,
            b'0'..=b'9' => exp[1] - b'0',
            c => {
                anyhow::bail!("invalid byte in checksum '{expected}' @ {ind}: {c}");
            }
        };

        if digest[ind] != cur {
            anyhow::bail!("checksum mismatch, expected '{expected}'");
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
#[allow(unsafe_code)]
pub fn is_powershell_parent() -> bool {
    if !cfg!(target_os = "linux") {
        // Making the assumption that no one on MacOS or any of the *BSDs uses powershell...
        return false;
    }

    // SAFETY: no invariants to uphold
    let mut parent_id = Some(unsafe { libc::getppid() });

    while let Some(ppid) = parent_id {
        let Ok(cmd) = std::fs::read_to_string(format!("/proc/{ppid}/cmdline")) else {
            break;
        };

        let Some(proc) = cmd
            .split('\0')
            .next()
            .and_then(|path| path.split('/').last())
        else {
            break;
        };

        if proc == "pwsh" {
            return true;
        }

        let Ok(status) = std::fs::read_to_string(format!("/proc/{ppid}/status")) else {
            break;
        };

        for line in status.lines() {
            let Some(ppid) = line.strip_prefix("PPid:\t") else {
                continue;
            };

            parent_id = ppid.parse().ok();
            break;
        }
    }

    false
}

#[cfg(target_family = "windows")]
mod win_bindings;

#[cfg(target_family = "windows")]
#[allow(unsafe_code)]
pub fn is_powershell_parent() -> bool {
    use std::os::windows::ffi::OsStringExt as _;
    use win_bindings::*;

    let mut pid = Some(-1 /* NtCurrentProcess */);

    unsafe {
        let reset = |fname: &mut [u16]| {
            let ustr = &mut *fname.as_mut_ptr().cast::<UnicodeString>();
            ustr.length = 0;
            ustr.maximum_length = MaxPath as _;
            ustr.buffer = fname
                .as_mut_ptr()
                .byte_offset(std::mem::size_of::<UnicodeString>() as _);
        };

        // The API for this is extremely irritating, the struct and string buffer
        // need to be the same :/
        let mut file_name = [0u16; MaxPath as usize + std::mem::size_of::<UnicodeString>() / 2];

        while let Some(ppid) = pid {
            let mut basic_info = std::mem::MaybeUninit::<ProcessBasicInformation>::uninit();
            let mut length = 0;
            if dbg!(nt_query_information_process(
                ppid,
                Processinfoclass::ProcessBasicInformation,
                basic_info.as_mut_ptr().cast(),
                std::mem::size_of::<ProcessBasicInformation>() as _,
                &mut length,
            )) != StatusSuccess
            {
                return false;
            }

            if length != std::mem::size_of::<ProcessBasicInformation>() as u32 {
                return false;
            }

            let basic_info = basic_info.assume_init();
            reset(&mut file_name);

            if dbg!(nt_query_information_process(
                basic_info.inherited_from_unique_process_id as _,
                Processinfoclass::ProcessImageFileName,
                file_name.as_mut_ptr().cast(),
                (file_name.len() * 2) as _,
                &mut length,
            )) != StatusSuccess
            {
                return false;
            }

            let ustr = &*file_name.as_ptr().cast::<UnicodeString>();
            let os = std::ffi::OsString::from_wide(
                &file_name[std::mem::size_of::<UnicodeString>() * 2
                    ..std::mem::size_of::<UnicodeString>() * 2 + ustr.length as usize],
            );

            let path = os.to_string_lossy();
            dbg!(&path);
            let p = std::path::Path::new(path.as_ref());
            if p.file_stem() == Some(std::ffi::OsStr::new("pwsh")) {
                return true;
            }

            pid = (basic_info.inherited_from_unique_process_id != 0)
                .then_some(basic_info.inherited_from_unique_process_id as isize);
        }

        false
    }
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore = "call when actually run from powershell"]
    fn is_powershell_true() {
        assert!(super::is_powershell_parent());
    }

    #[test]
    #[ignore = "call when not actually run from powershell"]
    fn is_powershell_false() {
        assert!(!super::is_powershell_parent());
    }
}
