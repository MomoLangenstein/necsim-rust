use std::{env, fs::File, io::Write, path};

use rustc_version::Channel;

/// Based on Sebastian Waisbrot's MIT licensed `rustc-version-runtime` crate:
///  https://github.com/seppo0010/rustc-version-runtime-rs
fn main() {
    let mut path = path::PathBuf::from(env::var_os("OUT_DIR").unwrap());
    path.push("rustc_version.rs");
    let mut file = File::create(&path).unwrap();

    writeln!(
        file,
        "use rustc_version::{{Channel, LlvmVersion, Version, VersionMeta}};
use semver::{{BuildMetadata, Prerelease}};\n"
    )
    .unwrap();
    let version = rustc_version::version_meta().expect("Failed to read the rustc version.");

    writeln!(
        file,
        "/// Returns the `rustc` `SemVer` version and additional metadata
/// like the git short hash and build date.
pub fn version_meta() -> VersionMeta {{
    VersionMeta {{
        semver: semver::Version {{
            major: {major},
            minor: {minor},
            patch: {patch},
            pre: Prerelease::new(\"{pre}\").unwrap(),
            build: BuildMetadata::new(\"{build}\").unwrap(),
        }},
        commit_hash: {commit_hash},
        commit_date: {commit_date},
        build_date: {build_date},
        channel: Channel::{channel},
        host: \"{host}\".to_owned(),
        short_version_string: \"{short_version_string}\".to_owned(),
        llvm_version: {llvm_version},
    }}
}}",
        major = version.semver.major,
        minor = version.semver.minor,
        patch = version.semver.patch,
        pre = version.semver.pre,
        build = version.semver.build,
        commit_hash = version
            .commit_hash
            .map(|h| format!("Some(\"{h}\".to_owned())"))
            .unwrap_or_else(|| "None".to_owned()),
        commit_date = version
            .commit_date
            .map(|h| format!("Some(\"{h}\".to_owned())"))
            .unwrap_or_else(|| "None".to_owned()),
        build_date = version
            .build_date
            .map(|h| format!("Some(\"{h}\".to_owned())"))
            .unwrap_or_else(|| "None".to_owned()),
        channel = match version.channel {
            Channel::Dev => "Dev",
            Channel::Nightly => "Nightly",
            Channel::Beta => "Beta",
            Channel::Stable => "Stable",
        },
        host = version.host,
        short_version_string = version.short_version_string,
        llvm_version = version
            .llvm_version
            .map(|h| format!(
                "Some(LlvmVersion {{
            major: {major},
            minor: {minor},
        }})",
                major = h.major,
                minor = h.minor
            ))
            .unwrap_or_else(|| "None".to_owned()),
    )
    .unwrap();
}
