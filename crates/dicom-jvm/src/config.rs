use crate::{Capabilities, Error};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

/// Validated deployment configuration. Paths/options never originate from requests.
pub struct JvmConfig {
    pub(crate) libjvm: PathBuf,
    pub(crate) options: Vec<String>,
    pub(crate) timeout: Duration,
    pub(crate) shutdown_timeout: Duration,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    api_version: u32,
    facade_version: String,
    capabilities: Capabilities,
    artifacts: BTreeMap<String, String>,
}

impl JvmConfig {
    /// Returns None only when JNI is explicitly disabled or unset.
    pub fn from_env() -> Result<Option<Self>, Error> {
        if !boolean("VIEWER_JNI_ENABLED", false)? {
            return Ok(None);
        }
        let home = env::var_os("PACS_JAVA_HOME").map(PathBuf::from);
        let explicit_lib = env::var_os("PACS_LIBJVM_PATH").map(PathBuf::from);
        let relative = if cfg!(windows) {
            "bin/server/jvm.dll"
        } else {
            "lib/server/libjvm.so"
        };
        let (home, libjvm) = match (home, explicit_lib) {
            (Some(home), explicit) => {
                let expected = canonical(&home.join(relative))?;
                if let Some(lib) = explicit
                    && canonical(&lib)? != expected
                {
                    return Err(Error::RuntimeConfig);
                }
                (canonical(&home)?, expected)
            }
            (None, Some(lib)) => {
                let lib = canonical(&lib)?;
                let home = lib
                    .ancestors()
                    .nth(3)
                    .ok_or(Error::RuntimeConfig)?
                    .to_owned();
                if canonical(&home.join(relative))? != lib {
                    return Err(Error::RuntimeConfig);
                }
                (home, lib)
            }
            _ => return Err(Error::RuntimeConfig),
        };
        let release = fs::read_to_string(home.join("release")).map_err(|_| Error::RuntimeConfig)?;
        let version = release
            .lines()
            .find_map(|line| line.strip_prefix("JAVA_VERSION="))
            .ok_or(Error::RuntimeConfig)?
            .trim_matches('"');
        if version.split(['.', '-', '+']).next() != Some("25") {
            return Err(Error::JavaVersion);
        }
        // Invocation must not inherit arbitrary agent/JVM options from the launcher.
        for key in ["JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS"] {
            if env::var_os(key).is_some_and(|value| !value.is_empty()) {
                return Err(Error::RuntimeConfig);
            }
        }
        let distribution = canonical(&required_path("PACS_FACADE_DIR")?)?;
        let classpath = verify_distribution(&distribution)?;
        let temp = canonical(&required_path("PACS_JVM_TEMP_DIR")?)?;
        let native = canonical(&required_path("PACS_JVM_NATIVE_DIR")?)?;
        if !temp.is_dir() || !native.is_dir() {
            return Err(Error::RuntimeConfig);
        }
        let xms = number("PACS_JVM_XMS_MB")?;
        let xmx = number("PACS_JVM_XMX_MB")?;
        let direct = number("PACS_JVM_DIRECT_MB")?;
        if xms > xmx {
            return Err(Error::RuntimeConfig);
        }
        let mut options = vec![
            format!("-Djava.class.path={classpath}"),
            "-Djava.awt.headless=true".into(),
            format!("-Djava.io.tmpdir={}", path_text(&temp)?),
            format!("-Djava.library.path={}", path_text(&native)?),
            format!("-Xms{xms}m"),
            format!("-Xmx{xmx}m"),
            format!("-XX:MaxDirectMemorySize={direct}m"),
            "-XX:+ExitOnOutOfMemoryError".into(),
            // Rust owns process signals; Java must not replace the HTTP shutdown handlers.
            "-Xrs".into(),
        ];
        if boolean("PACS_JVM_CHECK_JNI", cfg!(debug_assertions))? {
            options.push("-Xcheck:jni".into());
        }
        Ok(Some(Self {
            libjvm,
            options,
            timeout: Duration::from_millis(number("PACS_JVM_TIMEOUT_MS")?),
            shutdown_timeout: Duration::from_millis(number("PACS_JVM_SHUTDOWN_MS")?),
        }))
    }
}

fn required_path(name: &str) -> Result<PathBuf, Error> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(Error::RuntimeConfig)
}

fn number(name: &str) -> Result<u64, Error> {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .ok_or(Error::RuntimeConfig)
}

fn boolean(name: &str, default: bool) -> Result<bool, Error> {
    match env::var(name) {
        Ok(value) if value == "true" => Ok(true),
        Ok(value) if value == "false" => Ok(false),
        Err(env::VarError::NotPresent) => Ok(default),
        _ => Err(Error::RuntimeConfig),
    }
}

fn canonical(path: &Path) -> Result<PathBuf, Error> {
    let path = fs::canonicalize(path).map_err(|_| Error::RuntimeConfig)?;
    // Java classpaths do not understand Windows verbatim path prefixes.
    let text = path.to_str().ok_or(Error::RuntimeConfig)?;
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        Ok(PathBuf::from(format!(r"\\{unc}")))
    } else if let Some(normal) = text.strip_prefix(r"\\?\") {
        Ok(PathBuf::from(normal))
    } else {
        Ok(path)
    }
}

fn path_text(path: &Path) -> Result<&str, Error> {
    path.to_str()
        .filter(|text| !text.contains('\0'))
        .ok_or(Error::RuntimeConfig)
}

fn verify_distribution(root: &Path) -> Result<String, Error> {
    let bytes = fs::read(root.join("manifest.json")).map_err(|_| Error::Artifact)?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::Artifact);
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| Error::Artifact)?;
    if manifest.schema_version != 1
        || manifest.api_version != 1
        || manifest.facade_version != "0.1.0"
        || !manifest.capabilities.is_foundation()
    {
        return Err(Error::Contract);
    }
    let dependencies = root.join("dependencies");
    if !dependencies.is_dir() {
        return Err(Error::Artifact);
    }
    let mut files = BTreeMap::new();
    files.insert(
        "pacs-dicom-facade.jar".to_owned(),
        root.join("pacs-dicom-facade.jar"),
    );
    for entry in fs::read_dir(&dependencies).map_err(|_| Error::Artifact)? {
        let path = entry.map_err(|_| Error::Artifact)?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jar") || !path.is_file() {
            return Err(Error::Artifact);
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::Artifact)?;
        files.insert(format!("dependencies/{name}"), path);
    }
    if files.keys().ne(manifest.artifacts.keys()) {
        return Err(Error::Artifact);
    }
    let mut classpath = Vec::new();
    for (name, path) in files {
        let path = canonical(&path).map_err(|_| Error::Artifact)?;
        if !path.starts_with(root) {
            return Err(Error::Artifact);
        }
        let mut file = fs::File::open(&path).map_err(|_| Error::Artifact)?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 16384];
        loop {
            let length = file.read(&mut buffer).map_err(|_| Error::Artifact)?;
            if length == 0 {
                break;
            }
            hash.update(&buffer[..length]);
        }
        if format!("{:x}", hash.finalize()) != manifest.artifacts[&name] {
            return Err(Error::Artifact);
        }
        let text = path_text(&path)?;
        if text.contains(if cfg!(windows) { ';' } else { ':' }) {
            return Err(Error::Artifact);
        }
        classpath.push(text.to_owned());
    }
    Ok(classpath.join(if cfg!(windows) { ";" } else { ":" }))
}
