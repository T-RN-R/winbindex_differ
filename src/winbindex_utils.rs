//! Loads Winbindex metadata for a given file, and exposes operations on it.

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
struct Attribute {
    name: String,
    #[serde(alias = "destinationPath")]
    destination_path: String,
    #[serde(alias = "sourceName")]
    source_name: String,
    #[serde(alias = "importPath")]
    import_path: String,
    #[serde(alias = "sourcePath")]
    source_path: String,
}

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
struct AssemblyIdentity {
    name: String,
    version: String,
    #[serde(alias = "processorArchitecture")]
    processor_architecture: String,
    language: String,
    #[serde(alias = "buildType")]
    build_type: String,
    #[serde(alias = "publicKeyToken")]
    public_key_token: String,
    #[serde(alias = "versionScope")]
    version_scope: String,
}

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
struct Assembly {
    #[serde(alias = "assemblyIdentity")]
    assembly_identity: AssemblyIdentity,
    attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
struct UpdateInfo {
    arch: String,
    build: String,
    created: Number,
    title: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Build {
    #[serde(alias = "updateInfo")]
    update_info: UpdateInfo,
    assemblies: HashMap<String, Assembly>,
}
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
pub struct FileInfo {
    size: Number,
    md5: Option<String>,
    sha1: Option<String>,
    pub sha256: Option<String>,
    #[serde(alias = "machineType")]
    machine_type: Number,
    timestamp: Number,
    #[serde(alias = "virtualSize")]
    virtual_size: Option<Number>,
    version: Option<String>,
    description: Option<String>,
    #[serde(alias = "signingStatus")]
    signing_status: Option<String>,
    #[serde(alias = "signatureType")]
    signature_type: Option<String>,
    #[serde(alias = "signingDate")]
    signing_date: Option<Vec<String>>,
}
impl Default for FileInfo {
    fn default() -> Self {
        Self {
            size: Number::from(0),
            md5: None,
            sha1: None,
            sha256: None,
            machine_type: Number::from(0),
            timestamp: Number::from(0),
            virtual_size: None,
            version: None,
            description: None,
            signing_status: None,
            signature_type: None,
            signing_date: None,
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct WindowsVersion {
    pub builds: Option<HashMap<String, Build>>,
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct BinaryVersion {
    major: u32,
    minor: u32,
    patch: u32,
    build: u32,
}
impl BinaryVersion {
    pub fn parse(version_string: &str) -> Option<Self> {
        let parts: Vec<&str> = version_string.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        let build = parts[3].parse().ok()?;

        Some(Self {
            major,
            minor,
            patch,
            build,
        })
    }
}
impl PartialOrd for BinaryVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BinaryVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.build.cmp(&other.build))
    }
}

impl Default for BinaryVersion {
    fn default() -> Self {
        Self {
            major: 100_000,
            minor: 100_000,
            patch: 10000,
            build: 100_000,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Arch {
    X86,
    Amd64,
    Arm64,
    Arm,
    Invalid,
}
impl From<&str> for Arch {
    fn from(name: &str) -> Self {
        match name {
            "x86" => Self::X86,
            "amd64" => Self::Amd64,
            "arm64" => Self::Arm64,
            "arm" => Self::Arm,
            _ => Self::Invalid,
        }
    }
}
impl From<Arch> for String {
    fn from(val: Arch) -> Self {
        match val {
            Arch::Amd64 => "amd64".to_owned(),
            Arch::Arm64 => "arm64".to_owned(),
            Arch::Arm => "arm".to_owned(),
            Arch::X86 => "x86".to_owned(),
            Arch::Invalid => "Invalid".to_owned(),
        }
    }
}


#[derive(Debug)]
pub struct SymbolServerDownloadUrl {
    pub url: String,
}
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct WinbindexEntry {
    #[serde(alias = "fileInfo")]
    pub file_info: Option<FileInfo>,
    #[serde(alias = "windowsVersions")]
    pub windows_version: WindowsVersion,
    #[serde(skip)]
    pub repo: String,
    #[serde(skip)]
    pub name: String,
}
impl WinbindexEntry {
    pub fn get_binary_dlname(&self) -> String {
        format!("{}_{}", self.get_sha256(), self.get_name())
    }
    pub fn get_arch(&self) -> Arch {
        //https://learn.microsoft.com/en-us/dotnet/api/system.reflection.portableexecutable.machine?view=net-8.0
        match self.file_info.as_ref().unwrap().machine_type.as_u64().unwrap(){
            34404 => Arch::Amd64,
            332 => Arch::X86,
            452 => Arch::Arm,
            43620 => Arch::Arm64,
            _ => Arch::Invalid
        }
    }
    pub fn get_version(&self) -> BinaryVersion {
        let builds = &self.windows_version.builds;
        if builds.is_none() {
            return BinaryVersion::default();
        }

        for v in builds.as_ref().unwrap().values() {
            if let Some((__, asm)) = v.assemblies.iter().next() {
                return BinaryVersion::parse(asm.assembly_identity.version.as_str())
                    .unwrap_or_default();
            }
        }
        BinaryVersion::default()
    }
    pub fn get_name(&self) ->String {
        self.name.clone()
    }
    pub fn get_sha256(&self) -> String {
        return self
            .file_info
            .as_ref()
            .unwrap()
            .sha256
            .as_ref()
            .unwrap()
            .clone();
    }
    pub fn get_timestamp(&self) -> Number {
        self.file_info.as_ref().unwrap().timestamp.clone()
    }
    pub fn get_download_url(&self) -> Option<SymbolServerDownloadUrl> {
        let timestamp: Number = self.get_timestamp();
        // TODO: Handle the cases where virual_size is None
        let image_size = self.file_info.as_ref()?.virtual_size.clone()?;

        // Format timestamp as hexadecimal and pad it to at least 8 characters
        let timestamp_hex = format!("{:08X}", timestamp.as_i64()?);

        // Format image_size as hexadecimal
        let image_size_hex = format!("{:x}", image_size.as_i64()?);

        // Combine both parts to create the file_id
        let file_id = format!("{timestamp_hex}{image_size_hex}");
        let name = self.get_name();
        let url = format!(
            "https://msdl.microsoft.com/download/symbols/{name}/{file_id}/{name}"
        );

        Some(SymbolServerDownloadUrl { url })
    }

    fn set_sha256(&mut self, sha256: String) {
        if self.file_info.is_none() {
            self.file_info = Some(FileInfo::default());
        }
        self.file_info.as_mut().unwrap().sha256 = Some(sha256);
    }
}

pub struct WinbindexFileData {
    pub data: HashMap<String, WinbindexEntry>,
}


impl WinbindexFileData {
    pub const fn new(data: HashMap<String, WinbindexEntry>) -> Self {
        Self { data }
    }

    pub fn find_previous_for_entry(&self, entry: &WinbindexEntry) -> Option<WinbindexEntry> {
        let mut by_version: HashMap<BinaryVersion, WinbindexEntry> = HashMap::new();
        for (_k, v) in self.data.clone() {
            if v.get_arch() == entry.get_arch() {
                by_version.insert(v.get_version(), v);
            }
        }
        let mut v: Vec<_> = by_version.keys().cloned().collect();
        v.sort();
        let position = v.iter().position(|r| r == &entry.get_version())?;

        if position == 0 {
            return None;
        }
        let prev_idx = position - 1;
        let prev_ver = v.get(prev_idx)?;
        let prev_entry = by_version.get(prev_ver)?;

        Some(prev_entry.clone())
    }
}
#[derive(Debug)]
pub enum WinbindexError {
    FileOpen(PathBuf),
    //FileRead,
    Gzip,
    InvalidWinbindexEntryFormatting(serde_json::Error),
    InvalidOsString
}

pub struct Winbindex {
    repo_path: PathBuf,
    data_path: PathBuf,
}

impl Winbindex {
    pub fn new(repo_path: &str, data_path: &str) -> Self {
        return Self {
            repo_path: Path::new(repo_path).to_path_buf(),
            data_path: Path::new(data_path).to_path_buf(),
        };
    }
    pub fn load_file(
        &self,
        file_name: &str,
        windbindex_type: &str,
    ) -> Result<WinbindexFileData, WinbindexError> {
        let file_path = self
            .repo_path
            .join(&self.data_path)
            .join(format!("{}{}", file_name, ".json.gz"));
        println!("Loading file {}", file_path.to_str().ok_or_else(||WinbindexError::InvalidOsString)?);
        let file = File::open(&file_path).map_err(|_err| WinbindexError::FileOpen(file_path))?;
        let mut gz_buf = String::new();
        //let read_to_end = File::read_to_end(&mut file, &mut gz_buf).map_err(|err|WinbindexError::FileRead)?;
        let _gz_decoded = GzDecoder::new(file)
            .read_to_string(&mut gz_buf)
            .map_err(|_err| WinbindexError::Gzip)?;
        let mut json: HashMap<String, WinbindexEntry> = serde_json::from_str(&gz_buf)
            .map_err(WinbindexError::InvalidWinbindexEntryFormatting)?;
        for (k, value) in &mut json {
            value.repo = windbindex_type.to_string();
            value.set_sha256(k.clone());
            value.name = file_name.to_string();
        }
        let mut cleaned_json: HashMap<String, WinbindexEntry> = HashMap::new();

        for (k, v) in json
            .into_iter()
            .filter(|(ref _k, ref v)| v.file_info.is_some())
        {
            cleaned_json.insert(k, v);
        }

        Ok(WinbindexFileData::new(cleaned_json))
    }
}
