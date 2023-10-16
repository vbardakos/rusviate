use std::env::consts::{FAMILY, OS, ARCH};
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::{Path, PathBuf};
use error_stack::{Result, Report, ResultExt};


const LOCAL_SOCKET: SocketAddr = SocketAddr::V4(
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8079)
);

#[derive(Debug)]
pub enum DatabaseBuildError {
    Generic,
    SysArchitectureError
}

impl Display for DatabaseBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database Failed to build")
    }
}

impl std::error::Error for DatabaseBuildError {}

trait BuildExt {
    type Output;
    fn build(self) -> Result<Self::Output, DatabaseBuildError>;
}

pub trait SocketAddrExt<T> {
    fn socket(self, input: T) -> Self;
}

#[derive(Debug)]
pub struct Database {
}

#[derive(Debug, Clone, Default)]
pub struct DatabaseBuilder<I, V> {
    socket_addr: Option<SocketAddr>,
    target: I,
    binary: Option<PathBuf>,
    data: Option<PathBuf>,
    grcp_port: Option<u16>,
    version: V,
    system: System,
}

/// SysOptions
#[derive(Debug, Clone)]
pub enum System {
    Any,
    MacOS,
    WindowsX86_64,
    WindowsAArch64,
    LinuxX86_64,
    LinuxAArch64,
}

/// Versioning
#[derive(Debug, Clone, Default)]
pub struct NoVersion;
#[derive(Debug, Clone)]
pub struct Version<T: Into<String>>(T);

/// Weaviate Image target
#[derive(Debug, Clone, Default)]
pub struct NoImage;

#[derive(Debug, Clone)]
pub enum Image<T> where T: Into<String> {
    Local(T),
    Url(T),
}

impl Default for System {
    fn default() -> Self {
        System::Any
    }
}

impl AsRef<str> for System {
    fn as_ref(&self) -> &str {
        match self {
            System::Any => "unknown",
            System::MacOS => "darwin-all",
            System::LinuxX86_64 => "linux-amd64",
            System::LinuxAArch64 => "linux-arm64",
            System::WindowsX86_64 => "windows-amd64",
            System::WindowsAArch64 => "windows-arm64"
        }
    }
}

impl BuildExt for System {
    type Output = System;

    fn build(self) -> Result<Self::Output, DatabaseBuildError> {
        match self {
            System::Any => {
                let sys = match (FAMILY, OS, ARCH) {
                    ("unix", "macos", _) => System::MacOS,
                    ("windows", _, "x86_64") => System::WindowsX86_64,
                    ("windows", _, "aarch64") => System::WindowsAArch64,
                    ("unix", _, "x86_64") => System::LinuxX86_64,
                    ("unix", _, "aarch64") => System::LinuxAArch64,
                    (_, _, other) => {
                        let message = format!("Unsupported architecture: {other:?}");
                        return Err(Report::new(DatabaseBuildError::SysArchitectureError))
                            .attach_printable(message)
                    }
                };
                Ok(sys)
            },
            other => Ok(other)
        }
    }
}

impl<T> BuildExt for Version<T>
    where T: Into<String>
{
    type Output = ();

    fn build(self) -> Result<Self::Output, DatabaseBuildError> {
        let raw_tag = self.0.into();
        if raw_tag == "latest" {
            
        }
        Ok(())
    }
}

impl DatabaseBuilder<NoImage, NoVersion> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<V> DatabaseBuilder<NoImage, V> {
    pub fn target_file<P: AsRef<Path> + Into<String>>(self, image: P) -> DatabaseBuilder<Image<P>, V> {
        DatabaseBuilder {
            target: Image::Local(image),
            socket_addr: self.socket_addr,
            grcp_port: self.grcp_port,
            binary: self.binary,
            data: self.data,
            version: self.version,
            system: self.system,
        }
    }

    pub fn target_url<U: Into<String>>(self, image: U) -> DatabaseBuilder<Image<U>, V> {
        DatabaseBuilder {
            target: Image::Url(image),
            socket_addr: self.socket_addr,
            grcp_port: self.grcp_port,
            binary: self.binary,
            data: self.data,
            version: self.version,
            system: self.system,
        }
    }
}

impl<I> DatabaseBuilder<I, NoVersion> {
    pub fn image_version<T: Into<String>>(self, version: T) -> DatabaseBuilder<I, Version<T>> {
        DatabaseBuilder {
            version: Version(version),
            socket_addr: self.socket_addr,
            grcp_port: self.grcp_port,
            binary: self.binary,
            data: self.data,
            target: self.target,
            system: self.system,
        }
    }

    pub fn latest_version<'a>(self) -> DatabaseBuilder<I, Version<&'a str>> {
        self.image_version("latest")
    }
}

impl<I, V> DatabaseBuilder<I, V> {
    pub fn binary_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.binary.replace(path.into());
        self
    }

    pub fn data_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.data.replace(path.into());
        self
    }

    pub fn grcp(mut self, port: u16) -> Self {
        self.grcp_port.replace(port);
        self
    }

    pub fn system(mut self, sys: System) -> Self {
        self.system = sys;
        self
    }
}

impl<I, V> SocketAddrExt<&str> for DatabaseBuilder<I, V> {
    fn socket(mut self, input: &str) -> Self {
        self.socket_addr = input.parse().ok();
        self
    }
}

impl<I, V> SocketAddrExt<(IpAddr, u16)> for DatabaseBuilder<I, V> {
    fn socket(mut self, input: (IpAddr, u16)) -> Self {
        self.socket_addr.replace(SocketAddr::new(input.0, input.1));
        self
    }
}

impl<I, V> SocketAddrExt<SocketAddr> for DatabaseBuilder<I, V> {
    fn socket(mut self, input: SocketAddr) -> Self {
        self.socket_addr.replace(input);
        self
    }
}

// impl<T> BuildExt for DatabaseBuilder<NoImage, Version<T>>
//     where T: Into<String>
// {
//     fn build(self) -> Result<Database, DatabaseBuildError> {
//         let version = match self.version.0 {
//             "latest" => {
// 
//             },
//             raw_tag => {
// 
//             }
//         };
// 
//         Ok(Database {})
//     }
// }


#[test]
fn check() {
    let builder = DatabaseBuilder::new()
        .binary_dir("./binary")
        .grcp(100)
        .image_version("v1.21.1")
        .target_file("this")
        .socket("127.0.0.1:80000");

    println!("{:#?}", builder);
}

