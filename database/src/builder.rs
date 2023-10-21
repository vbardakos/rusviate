use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use crate::{errors::DbBuilderErrors};
use error_stack::{Report, Result, ResultExt};
use std::env::consts::{ARCH, OS};
use std::fs;
use simple_home_dir::expand_tilde;


const DATA: &str = "~/.local/share/weaviate";
const ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8079);
const BIN: &str = "~/.cache/weaviate-embedded/";
const GIT: &str = "https://github.com/weaviate/weaviate/releases/download";
const VERSION: &str = "v1.21.1";

#[derive(Debug, Clone)]
pub enum Image<'a> {
    Local(Cow<'a, Path>),
    Url(Cow<'a, str>),
}

/// Database Builder Version TypeStates
#[derive(Debug, Clone, Default)]
struct NoVersion;

#[derive(Debug, Clone, Default)]
struct VersionWrapper<'a>(Cow<'a, str>);

type VersionedDbBuilder<'a> = EmbeddedDatabaseBuilder<'a, VersionWrapper<'a>>;

/// build process
/// create bin & data dirs
/// create actual bin
/// validate version value
/// check system is not windows
/// add env defaults & override with extras
#[derive(Debug, Clone, Default)]
pub struct EmbeddedDatabaseBuilder<'a, V> {
    version: V,
    binary: Option<PathBuf>,
    data: Option<Cow<'a, Path>>,
    addr: Option<SocketAddr>,
    grcp: Option<u16>,
    target: Option<Image<'a>>,
    conn_retries: Option<(usize, usize)>,
    extras: Option<HashMap<&'a str, &'a str>>
}

impl<'a> VersionWrapper<'a> {
    fn new<S: Into<Cow<'a, str>>>(t: S) -> Self {
        let tag = t.into();
        if tag.starts_with("v") || tag == "latest" {
            return VersionWrapper(tag)
        }
        VersionWrapper(Cow::Owned("v".to_string() + tag.as_ref()))
    }
}

// add version
impl<'a> EmbeddedDatabaseBuilder<'a, NoVersion> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn latest_version(self) -> VersionedDbBuilder<'a> {
        self.version("latest")
    }

    pub fn default_version(self) -> VersionedDbBuilder<'a> {
        self.version(VERSION)
    }

    pub fn version<S: Into<Cow<'a, str>>>(self, tag: S) -> VersionedDbBuilder<'a> {
        EmbeddedDatabaseBuilder {
            version: VersionWrapper::new(tag),
            binary: self.binary,
            data: self.data,
            addr: self.addr,
            grcp: self.grcp,
            target: self.target,
            conn_retries: self.conn_retries,
            extras: self.extras,
        }
    }
}

pub trait TargetBuildExt<T> {
    fn target<U: Into<T>>(self, input: U) -> Self;
}

impl<'a, V> TargetBuildExt<Cow<'a, str>> for EmbeddedDatabaseBuilder<'a, V> {
    fn target<U: Into<Cow<'a, str>>>(mut self, url: U) -> Self {
        self.target.replace(Image::Url(url.into()));
        self
    }
}

impl<'a, V> TargetBuildExt<Cow<'a, Path>> for EmbeddedDatabaseBuilder<'a, V> {
    fn target<U: Into<Cow<'a, Path>>>(mut self, dir: U) -> Self {
        self.target.replace(Image::Local(dir.into()));
        self
    }
}

pub trait SocketAddrBuildExt<T> {
    fn socket(self, input: T) -> Self;
}

impl<'a, V> SocketAddrBuildExt<&'a str> for EmbeddedDatabaseBuilder<'a, V> {
    fn socket(mut self, input: &'a str) -> Self {
        self.addr = input.parse().ok();
        self
    }
}

impl<'a, V> SocketAddrBuildExt<SocketAddr> for EmbeddedDatabaseBuilder<'a, V> {
    fn socket(mut self, input: SocketAddr) -> Self {
        self.addr.replace(input);
        self
    }
}

impl<'a, V> SocketAddrBuildExt<(IpAddr, u16)> for EmbeddedDatabaseBuilder<'a, V> {
    fn socket(mut self, input: (IpAddr, u16)) -> Self {
        self.addr.replace(
            SocketAddr::new(input.0, input.1)
        );
        self
    }
}

impl<'a, V> SocketAddrBuildExt<IpAddr> for EmbeddedDatabaseBuilder<'a, V> {
    fn socket(mut self, input: IpAddr) -> Self {
        self.addr.replace(
            SocketAddr::new(input, ADDR.port())
        );
        self
    }
}

impl<'a, V> EmbeddedDatabaseBuilder<'a, V> {
    pub fn data_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.data.replace(Cow::Owned(path.into()));
        self
    }

    pub fn binary_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.binary.replace(path.into());
        self
    }

    pub fn grcp_port(mut self, port: u16) -> Self {
        self.grcp.replace(port);
        self
    }

    pub fn extras<T: Into<HashMap<&'a str, &'a str>>>(mut self, data: T) -> Self {
        self.extras.replace(data.into());
        self
    }

    fn get_system_extension() -> Result<&'a str, DbBuilderErrors> {
        let sys = match (OS, ARCH) {
            ("macos", _) => "darwin-all.zip",
            ("linux", "x86_64") => "linux-amd64.tar.gz",
            ("linux", "aarch64") => "linux-arm64.tar.gz",
            (os, arch) => {
                let message = format!("Unsupported System: {os}_{arch}");
                return Err(Report::new(DbBuilderErrors::InvalidSystem))
                    .attach_printable(message);
            }
        };
        Ok(sys)
    }
}

impl<'a> VersionedDbBuilder<'a> {
    fn create_directories(mut self) -> Result<(), DbBuilderErrors> {
        let binary: PathBuf = self.binary
            .as_ref()
            .or(expand_tilde(BIN).as_ref())
            .ok_or_else(|| DbBuilderErrors::InvalidDirectory)
            .attach_printable_lazy(|| "Failed to parse the Default Binary Directory")?
            .into();

        let data = self.data
            .unwrap_or(
                Cow::Owned(
                    expand_tilde(DATA)
                        .ok_or_else(|| DbBuilderErrors::InvalidDirectory)
                        .attach_printable_lazy(|| "Failed to parse the Default Data Directory")?
                )
            );

        fs::create_dir_all(binary.as_path())
            .change_context(DbBuilderErrors::Generic)
            .attach_printable(format!("Unable to create directory {binary:?}"))?;

        fs::create_dir_all(data.as_ref())
            .change_context(DbBuilderErrors::Generic)
            .attach_printable(format!("Unable to create directory {data:?}"))?;

        self.binary = Some(binary);
        self.data = Some(data);

        Ok(())
    }

    fn build_target(mut self) -> Result<(), DbBuilderErrors> {
        if self.target.is_some() {

            return Ok(())
        }
        let sys_extension = Self::get_system_extension()?;
        let version = self.version.0;

        self.target.replace(Image::Url(
            Cow::Owned(
                format!("{GIT}/{version}/weaviate-{version}-{sys_extension}")
            )
        ));
        Ok(())
    }
}

#[test]
fn check() {
    std::fs::create_dir_all("./this/that").unwrap();
}