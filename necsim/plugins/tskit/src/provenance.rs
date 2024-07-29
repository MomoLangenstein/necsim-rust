use std::{collections::HashMap, io};

use findshlibs::{SharedLibrary, TargetSharedLibrary};
use serde::Serialize;

/// tskit's provenance JSON schema format root for version 1.0.0
#[expect(clippy::module_name_repetitions)]
#[derive(Serialize)]
pub struct TskitProvenance {
    schema_version: String,
    software: TskitProvenanceSoftware,
    parameters: TskitProvenanceParameters,
    environment: TskitProvenanceEnvironment,
}

impl TskitProvenance {
    pub fn try_new() -> io::Result<Self> {
        Ok(Self {
            schema_version: "1.0.0".to_owned(),
            software: TskitProvenanceSoftware::try_new()?,
            parameters: TskitProvenanceParameters::new(),
            environment: TskitProvenanceEnvironment::try_new()?,
        })
    }
}

#[derive(Serialize)]
struct TskitProvenanceSoftware {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
}

impl TskitProvenanceSoftware {
    pub fn try_new() -> io::Result<Self> {
        let executable = std::env::current_exe()?.canonicalize()?;

        let output = std::process::Command::new(&executable).arg("-V").output()?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let mut version = version_str.split_whitespace();

        // Split a version string such as 'man 2.9.1' into 'man' and '2.9.1'
        Ok(Self {
            name: version
                .next()
                .map_or_else(|| executable.to_string_lossy().into_owned(), str::to_owned),
            version: version
                .next()
                .map_or_else(|| "???".to_owned(), str::to_owned),
            commit: match git_version::git_version!(fallback = "unknown") {
                "unknown" => None,
                version => Some(version.to_owned()),
            },
        })
    }
}

#[derive(Serialize)]
struct TskitProvenanceParameters {
    args: Vec<String>,
}

impl TskitProvenanceParameters {
    pub fn new() -> Self {
        Self {
            args: std::env::args().collect(),
        }
    }
}

#[derive(Serialize)]
struct TskitProvenanceEnvironment {
    os: TskitProvenanceEnvironmentOs,
    #[expect(clippy::zero_sized_map_values)]
    libraries: HashMap<String, TskitProvenanceEnvironmentLibrary>,
    #[serde(with = "self::rustc_version::VersionMetaDef")]
    rustc: ::rustc_version::VersionMeta,
}

impl TskitProvenanceEnvironment {
    pub fn try_new() -> io::Result<Self> {
        #[expect(clippy::zero_sized_map_values)]
        let mut libraries = HashMap::new();

        // Create a map of all dynamically loaded libraries
        TargetSharedLibrary::each(|lib| {
            if let Ok(library) = TskitProvenanceEnvironmentLibrary::try_new(lib.name()) {
                libraries.insert(lib.name().to_string_lossy().into_owned(), library);
            }
        });

        Ok(Self {
            os: TskitProvenanceEnvironmentOs::try_new()?,
            libraries,
            rustc: rustc_version::version_meta(),
        })
    }
}

#[derive(Serialize)]
struct TskitProvenanceEnvironmentOs {
    system: String,
    node: String,
    release: String,
    version: String,
    machine: String,
}

impl TskitProvenanceEnvironmentOs {
    pub fn try_new() -> io::Result<Self> {
        let uname = uname::uname()?;

        Ok(Self {
            system: uname.sysname,
            node: uname.nodename,
            release: uname.release,
            version: uname.version,
            machine: uname.machine,
        })
    }
}

#[derive(Serialize)]
struct TskitProvenanceEnvironmentLibrary {}

impl TskitProvenanceEnvironmentLibrary {
    #[expect(clippy::unnecessary_wraps)]
    pub fn try_new(_library: &std::ffi::OsStr) -> io::Result<Self> {
        // TODO: Future work might deduce version information etc.

        Ok(Self {})
    }
}

mod rustc_version {
    include!(concat!(env!("OUT_DIR"), "/rustc_version.rs"));

    #[derive(serde::Serialize)]
    #[serde(remote = "rustc_version::VersionMeta")]
    pub(super) struct VersionMetaDef {
        pub semver: Version,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub commit_hash: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub commit_date: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub build_date: Option<String>,
        #[serde(serialize_with = "serialize_channel")]
        pub channel: Channel,
        pub host: String,
        pub short_version_string: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "serialize_llvm_version")]
        pub llvm_version: Option<LlvmVersion>,
    }

    #[expect(clippy::trivially_copy_pass_by_ref)]
    fn serialize_channel<S: serde::Serializer>(
        channel: &Channel,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{channel:?}"))
    }

    fn serialize_llvm_version<S: serde::Serializer>(
        llvm_version: &Option<LlvmVersion>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if let Some(llvm_version) = llvm_version {
            serializer.collect_str(llvm_version)
        } else {
            serializer.serialize_none()
        }
    }
}
