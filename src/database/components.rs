use crate::database::errors::BuildErrors;
use error_stack::{FutureExt, Report, Result, ResultExt};
use octocrab::params::actions::Visibility::Selected;
use regex::Regex;
use simple_home_dir::expand_tilde;
use std::convert::Into;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::Path;
use std::{
    borrow::Cow,
    collections::HashMap,
    env::consts::{ARCH, FAMILY, OS},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

/// SysOptions
#[derive(Debug, Clone, Copy)]
pub enum System {
    MacOS,
    WindowsX86_64,
    WindowsAArch64,
    LinuxX86_64,
    LinuxAArch64,
}

/// Weaviate Versioning
#[derive(Debug, Clone, Default)]
struct NoVersion;

#[derive(Debug, Clone)]
struct Version<'a>(Cow<'a, str>);

/// Weaviate Target Image
#[derive(Debug, Clone, Default)]
struct NoImage;

#[derive(Debug, Clone)]
struct Image<T>(T);

#[derive(Debug, Clone, Default)]
pub struct DatabaseBuilder<'a, I, V> {
    bin: Option<PathBuf>,
    data: Option<PathBuf>,
    target: I,
    version: V,
    system: Option<System>,
    addr: Option<SocketAddr>,
    grcp: Option<u16>,
    extra: Option<HashMap<&'a str, &'a str>>,
}

impl AsRef<str> for System {
    fn as_ref(&self) -> &str {
        match self {
            Self::MacOS => "darwin-all",
            Self::LinuxX86_64 => "linux-amd64",
            Self::LinuxAArch64 => "linux-arm64",
            Self::WindowsX86_64 => "windows-amd64",
            Self::WindowsAArch64 => "windows-arm64",
        }
    }
}

impl System {
    fn parse() -> Result<Self, BuildErrors> {
        let sys = match (FAMILY, OS, ARCH) {
            ("unix", "macos", _) => System::MacOS,
            ("windows", _, "x86_64") => System::WindowsX86_64,
            ("windows", _, "aarch64") => System::WindowsAArch64,
            ("unix", _, "x86_64") => System::LinuxX86_64,
            ("unix", _, "aarch64") => System::LinuxAArch64,
            (_, _, other) => {
                let message = format!("Unsupported architecture: {other:?}");
                return Err(Report::new(BuildErrors::UnknownSystemError)).attach_printable(message);
            }
        };
        Ok(sys)
    }

    fn extension(&self) -> &str {
        match self {
            Self::MacOS => "zip",
            _ => "tar.gz",
        }
    }
}

impl<'a> DatabaseBuilder<'a, NoImage, NoVersion> {
    pub fn new() -> Self {
        Self {
            target: NoImage,
            version: NoVersion,
            ..Default::default()
        }
    }
}

impl<'a, I> DatabaseBuilder<'a, I, NoVersion> {
    pub fn latest_version(self) -> DatabaseBuilder<'a, I, Version<'a>> {
        DatabaseBuilder {
            bin: self.bin,
            version: Version(Cow::Borrowed("latest")),
            system: self.system,
            addr: self.addr,
            grcp: self.grcp,
            target: self.target,
            data: self.data,
            extra: self.extra,
        }
    }

    pub fn version<T: Into<Cow<'a, str>>>(self, tag: T) -> DatabaseBuilder<'a, I, Version<'a>> {
        DatabaseBuilder {
            bin: self.bin,
            version: Version(tag.into()),
            system: self.system,
            addr: self.addr,
            grcp: self.grcp,
            target: self.target,
            data: self.data,
            extra: self.extra,
        }
    }
}

impl<'a, V> DatabaseBuilder<'a, NoImage, V> {
    pub fn target_file<P: Into<PathBuf>>(self, path: P) -> DatabaseBuilder<'a, Image<PathBuf>, V> {
        DatabaseBuilder {
            target: Image(path.into()),
            bin: self.bin,
            version: self.version,
            system: self.system,
            addr: self.addr,
            grcp: self.grcp,
            data: self.data,
            extra: self.extra,
        }
    }

    pub fn target_url<U: Into<String>>(self, url: U) -> DatabaseBuilder<'a, Image<String>, V> {
        DatabaseBuilder {
            target: Image(url.into()),
            bin: self.bin,
            version: self.version,
            system: self.system,
            addr: self.addr,
            grcp: self.grcp,
            data: self.data,
            extra: self.extra,
        }
    }
}

impl<'a, I, V> DatabaseBuilder<'a, I, V> {
    const BINARY: &'a str = "~/.cache/weaviate-embedded/";
    const DATA: &'a str = "~/.local/share/weaviate";
    const ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8079));

    pub fn system_type(mut self, sys: System) -> Self {
        self.system.replace(sys);
        self
    }

    pub fn data_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.data.replace(path.into());
        self
    }

    pub fn binary_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.bin.replace(path.into());
        self
    }

    pub fn grcp_port(mut self, port: u16) -> Self {
        self.grcp.replace(port);
        self
    }

    pub fn extras<T: Into<HashMap<&'a str, &'a str>>>(mut self, data: T) -> Self {
        self.extra.replace(data.into());
        self
    }

    fn build_params(&mut self) -> Result<(), BuildErrors> {
        self.system = self.system.or(Some(System::parse()?));
        self.addr = self.addr.or(Some(DatabaseBuilder::<I, V>::ADDR));
        self.grcp = self.grcp.or(Some(50060));

        self.bin = Some(
            self.bin
                .as_ref()
                .or(expand_tilde(Self::BINARY).as_ref())
                .ok_or_else(|| BuildErrors::DefaultPathError)
                .attach_printable_lazy(|| "Failed to parse the Default Binary Directory")?
                .into(),
        );

        self.data = Some(
            self.data
                .as_ref()
                .or(expand_tilde(Self::DATA).as_ref())
                .ok_or_else(|| BuildErrors::DefaultPathError)
                .attach_printable_lazy(|| "Failed to parse the Default Data Directory")?
                .into(),
        );

        Ok(())
    }
}

pub trait SocketAddrExt<T> {
    fn address(self, input: T) -> Self;
}

impl<'a, I, V> SocketAddrExt<&'a str> for DatabaseBuilder<'a, I, V> {
    fn address(mut self, input: &'a str) -> Self {
        self.addr = input.parse().ok();
        self
    }
}

impl<'a, I, V> SocketAddrExt<(IpAddr, u16)> for DatabaseBuilder<'a, I, V> {
    fn address(mut self, input: (IpAddr, u16)) -> Self {
        self.addr.replace(SocketAddr::new(input.0, input.1));
        self
    }
}

impl<'a, I, V> SocketAddrExt<SocketAddr> for DatabaseBuilder<'a, I, V> {
    fn address(mut self, input: SocketAddr) -> Self {
        self.addr.replace(input);
        self
    }
}

trait VersionBuilderExt<'a> {
    fn build_version(&'a self) -> Result<Version<'a>, BuildErrors>;
}

impl<'a, T> VersionBuilderExt<'a> for DatabaseBuilder<'a, Image<T>, NoVersion>
where
    T: AsRef<str> + Into<String>,
{
    fn build_version(&'a self) -> Result<Version<'a>, BuildErrors> {
        let pattern = r"v\d.\d{1,2}.\d{1,2}?(-rc\.\d{1,2}|-beta\.\d{1,2}|-alpha\.\d{1,2}|$)?";
        let target_name = self.target.0.as_ref();

        Ok(Version(Cow::Borrowed(
            Regex::new(pattern)
                .change_context(BuildErrors::VersionValueError)
                .attach_printable("Failed to compile Version pattern")?
                .find(target_name)
                .ok_or_else(|| BuildErrors::VersionFetchError)
                .attach_printable_lazy(|| "Failed to fetch Version from target name")?
                .into(),
        )))
    }
}

impl<'a, I> DatabaseBuilder<'a, I, Version<'a>> {
    async fn fetch_latest(&self) -> Result<Version<'a>, BuildErrors> {
        let tag = octocrab::instance()
            .repos("weaviate", "weaviate")
            .releases()
            .get_latest()
            .change_context(BuildErrors::VersionFetchError)
            .attach_printable("Failed to Fetch the latest Version")
            .await?
            .tag_name;
        Ok(Version(Cow::Owned(tag)))
    }

    fn build_version(&mut self) -> Result<(), BuildErrors> {
        let tag = self.version.clone().0;

        self.version = if tag == "latest" {
            tokio::runtime::Runtime::new()
                .change_context(BuildErrors::VersionFetchError)
                .attach_printable("Version Fetching Runtime Error")?
                .block_on(self.fetch_latest())?
        } else {
            let re = Regex::new(
                r"^v\d\.\d{1,2}\.\d{1,2}?(-rc\.\d{1,2}|-beta\.\d{1,2}|-alpha\.\d{1,2}|$)$",
            )
            .change_context(BuildErrors::VersionValueError)
            .attach_printable("Failed to Compile Version Pattern")?;

            if !re.is_match(tag.as_ref()) {
                return Err(Report::new(BuildErrors::VersionValueError))
                    .attach_printable(format!("Version {tag:?} is invalid"));
            }
            Version(tag)
        };

        Ok(())
    }
}

trait CommonBuildExt {
    fn build_binary(self) -> Result<Database, BuildErrors>;
}

impl<'a> CommonBuildExt for DatabaseBuilder<'a, Image<PathBuf>, Version<'a>> {
    fn build_binary(self) -> Result<Database, BuildErrors> {
        if !(self.target.0.exists() & self.target.0.is_file()) {
            let msg = format!("Image path {:?} not found", self.target.0);
            return Err(Report::new(BuildErrors::ImageNotFoundError)).attach_printable(msg);
        }

        if !(self.target.0.ends_with(".zip") | self.target.0.ends_with(".tar.gz")) {
            let msg = format!("Image path {:?} is not a zip or tar.gz file", self.target.0);
            return Err(Report::new(BuildErrors::DefaultPathError)).attach_printable(msg);
        }

        todo!()
    }
}

impl<'a> DatabaseBuilder<'a, NoImage, Version<'a>> {
    const GIT: &'a str = "https://github.com/weaviate/weaviate/releases/download";

    pub fn build(mut self) -> Result<Self, BuildErrors> {
        self.build_params()?;
        self.build_version()?;
        let system = self.system.unwrap();

        let target = Image(format!(
            "{base}/{version}/weaviate-{version}-{sys}.{extension}",
            base = Self::GIT,
            version = self.version.0,
            sys = system.as_ref(),
            extension = system.extension()
        ));

        let builder = DatabaseBuilder {
            target,
            addr: self.addr,
            version: self.version,
            system: self.system,
            data: self.data,
            bin: self.bin,
            extra: self.extra,
            grcp: self.grcp,
        };

        todo!()
    }
}

pub struct Database {}

#[test]
fn check() {
    let path = Path::new("./new_dir/test");
    println!("{:?}", path.metadata());
}
