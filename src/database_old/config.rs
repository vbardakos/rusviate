// use std::{
//     env,
//     borrow::Cow,
//     default::Default,
//     fmt::Debug,
//     path::{Path},
//     net::{Ipv4Addr, SocketAddr, SocketAddrV4}
// };
//
// use std::collections::HashMap;
// use error_stack::{FutureExt, Report, Result, ResultExt};
// use regex::Regex;
// use crate::database_old::errors::{ImageConfigError};
// use async_trait::async_trait;
// use sha2::digest::Output;
//
//
// /// Configuration States
// #[derive(Debug, PartialEq)]
// pub struct Open;
//
// #[derive(Debug, PartialEq)]
// pub struct Locked;
//
//
// #[derive(Debug, PartialEq)]
// pub enum SysType {
//     Any,
//     MacOS,
//     WindowsX86_64,
//     WindowsAArch64,
//     LinuxX86_64,
//     LinuxAArch64,
// }
//
// #[derive(Debug, PartialEq)]
// pub enum Version<'a> {
//     Latest,
//     Tag(Cow<'a, str>),
// }
//
// #[derive(Debug, PartialEq)]
// pub enum ImageTarget<'a> {
//     Local(&'a Path),
//     Url(&'a str),
// }
//
// #[derive(Debug, PartialEq)]
// pub struct ImageOptions<'a, T: Sized> {
//     target: ImageTarget<'a>,
//     version: Version<'a>,
//     system: SysType,
//     state: T,
// }
//
// #[derive(Debug)]
// pub struct DatabaseOptions<'a, T> {
//     data_dir: Cow<'a, Path>,
//     pub binary_dir: Cow<'a, Path>,
//     pub image: ImageOptions<'a, T>,
//     pub socket_addr: SocketAddr,
//     grcp: u16,
//     extra: Option<HashMap<&'a str, &'a str>>,
//     state: T,
// }
//
// #[async_trait]
// pub trait ConfigBuilderExt {
//     type Output;
//     type Error;
//     async fn build(self) -> Result<Self::Output, Self::Error>;
// }
//
// impl AsRef<str> for SysType {
//     fn as_ref(&self) -> &str {
//         match self {
//             SysType::Any => "unknown",
//             SysType::MacOS => "darwin-all",
//             SysType::LinuxX86_64 => "linux-amd64",
//             SysType::LinuxAArch64 => "linux-arm64",
//             SysType::WindowsX86_64 => "windows-amd64",
//             SysType::WindowsAArch64 => "windows-arm64"
//         }
//     }
// }
//
// impl<'a> AsRef<str> for Version<'a> {
//     fn as_ref(&self) -> &str {
//         match self {
//             Version::Latest => "latest",
//             Version::Tag(v) => v.as_ref(),
//         }
//     }
// }
//
// #[async_trait]
// impl ConfigBuilderExt for SysType {
//     type Output = Self;
//     type Error = ImageConfigError;
//
//     async fn build(self) -> Result<Self::Output, Self::Error> {
//         match self {
//             Self::Any => {
//                 let sys = match (env::consts::FAMILY, env::consts::OS, env::consts::ARCH) {
//                     ("unix", "macos", _) => SysType::MacOS,
//                     ("windows", _, "x86_64") => SysType::WindowsX86_64,
//                     ("windows", _, "aarch64") => SysType::WindowsAArch64,
//                     ("unix", _, "x86_64") => SysType::LinuxX86_64,
//                     ("unix", _, "aarch64") => SysType::LinuxAArch64,
//                     (_, _, other) => {
//                         return Err(Report::new(ImageConfigError::UnsupportedArchError))
//                             .attach_printable(format!("Unsupported architecture: {other:?}"))
//                     }
//                 };
//                 Ok(sys)
//             },
//             other => Ok(other)
//         }
//     }
// }
//
// #[async_trait]
// impl<'a> ConfigBuilderExt for Version<'a> {
//     type Output = Self;
//     type Error = ImageConfigError;
//
//     async fn build(self) -> Result<Self::Output, Self::Error> {
//         let release_handler = octocrab::instance();
//
//         let tag = match self {
//             Version::Tag(raw) => {
//                 let re = Regex::new(
//                     r"^\d\.\d{1,2}\.\d{1,2}?(-rc\.\d{1,2}|-beta\.\d{1,2}|-alpha\.\d{1,2}|$)$"
//                 )
//                     .change_context(ImageConfigError::CompilationError)?;
//
//                 let tag = raw.strip_prefix("v")
//                     .or(Some(raw.as_ref()))
//                     .unwrap();
//
//                 if !re.is_match(tag) {
//                     return Err(Report::new(ImageConfigError::InvalidVersionError))
//                         .attach_printable(format!("Invalid Version Format: {raw}"))
//                 }
//
//                 let tag: Cow<str> = Cow::Owned("v".to_string() + tag);
//                 release_handler
//                     .repos("weaviate", "weaviate")
//                     .releases()
//                     .get_by_tag(tag.as_ref())
//                     .await
//                     .change_context(ImageConfigError::FetchVersionError)
//                     .map_err(|e|
//                         e.attach_printable(format!("Unable to Fetch Version: {tag}"))
//                     )?.tag_name
//             },
//             Version::Latest => {
//                 release_handler
//                     .repos("weaviate", "weaviate")
//                     .releases()
//                     .get_latest()
//                     .await
//                     .change_context(ImageConfigError::FetchVersionError)
//                     .map_err(|e|
//                         e.attach_printable("Cannot Fetch Latest Version")
//                     )?.tag_name
//             }
//         };
//
//         Ok(Version::Tag(Cow::Owned(tag)))
//     }
// }
//
// impl<'a> SysType {
//     fn extension(&self) -> &str {
//         match self {
//             SysType::MacOS => "zip",
//             _ => "tar.gz",
//         }
//     }
// }
//
// impl<'a> ImageTarget<'a> {
//     fn contains_extension(&self) -> bool {
//         let container = match self {
//             Self::Url(u) => u,
//             Self::Local(p) => p.to_str().unwrap_or_default(),
//         };
//         container.ends_with(".zip") | container.ends_with(".tar.gz")
//     }
//
//     fn exists(&self) -> bool {
//         match self {
//             Self::Url(url) => url.ends_with("zip") | url.ends_with("tar.gz"),
//             Self::Local(p) => p.exists() & p.is_file(),
//         }
//     }
// }
//
// impl<'a> Default for ImageOptions<'a, Open> {
//     fn default() -> Self {
//         ImageOptions {
//             target: ImageTarget::Url(""),
//             version: Version::Tag(Cow::Borrowed("v1.21.1")),
//             system: SysType::Any,
//             state: Open,
//         }
//     }
// }
//
// #[async_trait]
// impl<'a> ConfigBuilderExt for ImageTarget<'a> {
//     type Output = Self;
//     type Error = ImageConfigError;
//
//     async fn build(self) -> Result<Self::Output, Self::Error> {
//         match self {
//             ImageTarget::Url(url) if !(url.is_empty() | self.contains_extension()) => {
//                 return Err(Report::new(ImageConfigError::InvalidImageError))
//                     .attach_printable("Image Url does not contain zip or tar.gz extension")
//             },
//             ImageTarget::Local(path) if !(path.is_file() & self.contains_extension()) => {
//                 return Err(Report::new(ImageConfigError::InvalidImageError))
//                     .attach_printable("Image File does not contain zip or tar.gz extension")
//             },
//             other => Ok(other)
//         }
//     }
// }
//
// #[async_trait]
// impl<'a> ConfigBuilderExt for ImageOptions<'a, Open> {
//     type Output = ImageOptions<'a, Locked>;
//     type Error = ImageConfigError;
//
//     async fn build(self) -> Result<Self::Output, Self::Error> {
//         let target = self.target.build().await?;
//         let system = self.system.build().await?;
//
//         if target == ImageOptions::default().target {
//             return Ok(
//                 ImageOptions {
//                     target,
//                     version: self.version,
//                     system,
//                     state: Locked,
//                 }
//             )
//         }
//         Ok(
//
//         )
//     }
// }
//
// #[test]
// fn check() {
//     let x = ImageOptions::default();
//     x == ImageOptions::default();
// }
//
//     // async fn build(self) -> Result<Self::Output, Self::Error> {
//     //     if self.target.exists() {
//     //         Ok(
//     //             ImageOptions {
//     //                 target: self.target,
//     //                 version: self.version,
//     //                 system: self.system.build().await.unwrap_or(SysType::Any),
//     //                 state: Locked
//     //             }
//     //         )
//     //     };
//     //     match &self.target {
//     //         ImageTarget::Local(p) =>
//     //             Err(Report::new(ImageConfigError::InvalidImageError))
//     //                 .attach_printable(format!("Local file does not exist: {p:?}")),
//     //         _ => {
//     //             const BASE_URL: &str = "https://github.com/weaviate/weaviate/releases/download";
//     //             let version = self.version.build().await?;
//     //             let system = self.system.build().await?;
//     //
//     //             let target = ImageTarget::Url(
//     //                 format!(
//     //                     "{BASE_URL}/{version}/weaviate-{version}-{system}.{extension}",
//     //                     version = version.as_ref(),
//     //                     system = system.as_ref(),
//     //                     extension = system.extension(),
//     //                 ).as_str()
//     //             );
//     //
//     //             Ok(ImageOptions { target, version, system, state: Locked })
//     //         }
//     //     }
//     // }
//
//
//
// // impl<'a> TryFrom<ImageTarget<'a>> for ImageOptions<'a> {
// //     type Error = Report<ImageConfigError>;
// //
// //     fn try_from(value: ImageTarget<'a>) -> std::result::Result<Self, Self::Error> {
// //         match value {
// //             ImageTarget::Url(url) => {
// //                 if url.ends_with(".zip") | url.ends_with(".tar.gz") {
// //                     return Ok(
// //                         ImageOptions {
// //                             target: ImageTarget::Url(url),
// //                             version: "unknown".to_string(),
// //                             system: SysType::parse()?,
// //                         });
// //                 }
// //                 Err(Report::new(ImageConfigError::InvalidImageError))
// //                     .attach_printable("url should point directly to zip or tag.gz file")
// //             },
// //             ImageTarget::Local(input) => {
// //                 let file = Path::new(input.as_ref());
// //                 if file.exists() & file.is_file() {
// //                     if !(input.ends_with("zip") | input.ends_with("tar.gz")) {
// //                         return Err(Report::new(ImageConfigError::InvalidImageError))
// //                             .attach_printable("file should be zip or tag.gz")
// //                     }
// //                     return Ok(
// //                         ImageOptions {
// //                             target: ImageTarget::Local(input),
// //                             version: "unknown".to_string(),
// //                             system: SysType::parse()?,
// //                         })
// //                 }
// //                 Err(Report::new(ImageConfigError::InvalidImageError))
// //                     .attach_printable(format!("filepath is invalid: {input}"))
// //             }
// //         }
// //     }
// // }
// //
// // impl<'a> TryFrom<Version<'a>> for ImageOptions<'a> {
// //     type Error = Report<ImageConfigError>;
// //
// //     fn try_from(value: Version<'a>) -> std::result::Result<Self, Self::Error> {
// //         const BASE_URL: &str = "https://github.com/weaviate/weaviate/releases/download";
// //
// //         let version = String::try_from(value)?;
// //         let system = SysType::parse()?;
// //
// //         let url = format!(
// //             "{BASE_URL}/{version}/weaviate-{version}-{}.{}",
// //             system.as_ref(),
// //             if system == SysType::MacOS { "zip" } else { "tar.gz" }
// //         );
// //
// //         Ok(
// //             ImageOptions {
// //                 target: ImageTarget::Url(Cow::Owned(url)),
// //                 version,
// //                 system,
// //             }
// //         )
// //     }
// // }
// //
// // impl<'a> Default for DatabaseOptions<'a> {
// //     fn default() -> Self {
// //         let data = simple_home_dir::expand_tilde("~/.local/share/weaviate")
// //             .unwrap_or_default();
// //         let bin = simple_home_dir::expand_tilde("~/.cache/weaviate-embedded/")
// //             .unwrap_or_default();
// //         let v4_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8079);
// //
// //         DatabaseOptions {
// //             data_dir: Cow::Owned(data),
// //             binary_dir: Cow::Owned(bin),
// //             image: ImageOptions::default(),
// //             socket_addr: SocketAddr::V4(v4_addr),
// //             grcp: 50060,
// //             extra: None,
// //         }
// //     }
// // }
// //
// // pub trait BuildFromExt<T>: Sized {
// //     type Error;
// //
// //     fn build_from(input: T) -> Result<Self, Self::Error>;
// // }
// //
// // impl<'a> BuildFromExt<Version<'a>> for DatabaseOptions<'a> {
// //     type Error = ImageConfigError;
// //
// //     fn build_from(input: Version<'a>) -> Result<Self, Self::Error> {
// //         let image = ImageOptions::try_from(input)?;
// //         Ok(DatabaseOptions { image, ..Self::default() })
// //     }
// // }
// //
// // impl<'a> BuildFromExt<ImageTarget<'a>> for DatabaseOptions<'a> {
// //     type Error = ImageConfigError;
// //
// //     fn build_from(input: ImageTarget<'a>) -> Result<Self, Self::Error> {
// //         let image = ImageOptions::try_from(input)?;
// //         Ok(DatabaseOptions { image, ..Self::default() })
// //     }
// // }
