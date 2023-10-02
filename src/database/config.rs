use std::{
    env,
    borrow::{Cow, Borrow},
    default::Default,
    fmt::Debug,
    path::{Path},
    net::Ipv4Addr,
};

use std::collections::HashMap;
use error_stack::{Report, Result, ResultExt};
use regex;
use regex::Regex;
use crate::database::errors::{ImageConfigError};


#[derive(Debug, PartialEq)]
pub enum SysType {
    Any,
    MacOS,
    WindowsX86_64,
    WindowsAArch64,
    LinuxX86_64,
    LinuxAArch64,
}

#[derive(Debug)]
pub enum Version<'a> {
    Latest,
    Tag(&'a str),
}

#[derive(Debug)]
enum ImageTarget<'a> {
    Local(Cow<'a, str>),
    Url(Cow<'a, str>),
}

#[derive(Debug)]
pub struct ImageOptions<'a> {
    target: ImageTarget<'a>,
    version: String,
    system: SysType,
}

#[derive(Debug)]
pub struct DatabaseOptions<'a> {
    data_dir: Cow<'a, Path>,
    binary_dir: Cow<'a, Path>,
    image: ImageOptions<'a>,
    hostname: Cow<'a, str>,
    port: u32,
    grcp: u32,
    additional: Option<HashMap<&'a str, &'a str>>
}

impl AsRef<str> for SysType {
    fn as_ref(&self) -> &str {
        match self {
            SysType::Any => "unknown",
            SysType::MacOS => "darwin-all",
            SysType::LinuxX86_64 => "linux-amd64",
            SysType::LinuxAArch64 => "linux-arm64",
            SysType::WindowsX86_64 => "windows-amd64",
            SysType::WindowsAArch64 => "windows-arm64"
        }
    }
}

impl SysType {
    pub fn parse() -> Result<Self, ImageConfigError> {
        let sys = match (env::consts::FAMILY, env::consts::OS, env::consts::ARCH) {
            ("unix", "macos", _) => SysType::MacOS,
            ("windows", _, "x86_64") => SysType::WindowsX86_64,
            ("windows", _, "aarch64") => SysType::WindowsAArch64,
            ("unix", _, "x86_64") => SysType::LinuxX86_64,
            ("unix", _, "aarch64") => SysType::LinuxAArch64,
            (_, _, other) => {
                return Err(Report::new(ImageConfigError::UnsupportedArchError))
                    .attach_printable(format!("Unsupported architecture: {other:?}"))
            }
        };
        Ok(sys)
    }
}

impl<'a> TryFrom<Version<'a>> for String {
    type Error = Report<ImageConfigError>;

    fn try_from(value: Version<'a>) -> std::result::Result<Self, Self::Error> {
        match value {
            Version::Tag(raw) => {
                let re = Regex::new(
                    r"^\d\.\d{1,2}\.\d{1,2}?(-rc\.\d{1,2}|-beta\.\d{1,2}|-alpha\.\d{1,2}|$)$"
                )
                    .change_context(ImageConfigError::CompilationError)?;

                let tag = raw.strip_prefix("v")
                    .or(Some(raw))
                    .unwrap();

                if re.is_match(tag) {
                    return Ok("v".to_string() + tag)
                }

                Err(Report::new(ImageConfigError::InvalidVersionError))
                    .attach_printable(format!("Invalid Version Tag: {raw}"))
            },
            Version::Latest => {
                let latest = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let handler = octocrab::instance();
                        handler.repos("weaviate", "weaviate")
                            .releases()
                            .get_latest()
                            .await
                            .change_context(ImageConfigError::FetchVersionError)
                            .expect("cannot fetch latest version")
                            .tag_name
                    });

                Ok(latest)
            }
        }
    }
}

impl<'a> Default for ImageOptions<'a> {
    fn default() -> Self {
        ImageOptions::try_from(Version::Tag("v1.21.1"))
            .unwrap()
    }
}

impl<'a> TryFrom<ImageTarget<'a>> for ImageOptions<'a> {
    type Error = Report<ImageConfigError>;

    fn try_from(value: ImageTarget<'a>) -> std::result::Result<Self, Self::Error> {
        match value {
            ImageTarget::Url(url) => {
                if url.ends_with(".zip") | url.ends_with(".tar.gz") {
                    return Ok(
                        ImageOptions {
                            target: ImageTarget::Url(url),
                            version: "unknown".to_string(),
                            system: SysType::parse()?,
                        });
                }
                Err(Report::new(ImageConfigError::InvalidImageError))
                    .attach_printable("url should point directly to zip or tag.gz file")
            },
            ImageTarget::Local(input) => {
                let file = Path::new(input.as_ref());
                if file.exists() & file.is_file() {
                    if !(input.ends_with("zip") | input.ends_with("tar.gz")) {
                        return Err(Report::new(ImageConfigError::InvalidImageError))
                            .attach_printable("file should be zip or tag.gz")
                    }
                    return Ok(
                        ImageOptions {
                            target: ImageTarget::Local(input),
                            version: "unknown".to_string(),
                            system: SysType::parse()?,
                        })
                }
                Err(Report::new(ImageConfigError::InvalidImageError))
                    .attach_printable(format!("filepath is invalid: {input}"))
            }
        }
    }
}

impl<'a> TryFrom<Version<'a>> for ImageOptions<'a> {
    type Error = Report<ImageConfigError>;

    fn try_from(value: Version<'a>) -> std::result::Result<Self, Self::Error> {
        const BASE_URL: &str = "https://github.com/weaviate/weaviate/releases/download";

        let version = String::try_from(value)?;
        let system = SysType::parse()?;

        let url = format!(
            "{BASE_URL}/{version}/weaviate-{version}-{}.{}",
            system.as_ref(),
            if system == SysType::MacOS { "zip" } else { "tar.gz" }
        );

        Ok(
            ImageOptions {
                target: ImageTarget::Url(Cow::Owned(url)),
                version,
                system,
            }
        )
    }
}

impl<'a> Default for DatabaseOptions<'a> {
    fn default() -> Self {
        let data = simple_home_dir::expand_tilde("~/.local/share/weaviate")
            .unwrap_or_default();
        let bin = simple_home_dir::expand_tilde("~/.cache/weaviate-embedded/")
            .unwrap_or_default();

        DatabaseOptions {
            data_dir: Cow::Owned(data),
            binary_dir: Cow::Owned(bin),
            image: ImageOptions::default(),
            hostname: Cow::Owned(Ipv4Addr::LOCALHOST.to_string()),
            port: 8079,
            grcp: 50060,
            additional: None,
        }
    }
}

pub trait BuildFromExt<T>: Sized {
    type Error;

    fn build_from(input: T) -> Result<Self, Self::Error>;
}

impl<'a> BuildFromExt<Version<'a>> for DatabaseOptions<'a> {
    type Error = ImageConfigError;

    fn build_from(input: Version<'a>) -> Result<Self, Self::Error> {
        let image = ImageOptions::try_from(input)?;
        Ok(DatabaseOptions { image, ..Self::default() })
    }
}

impl<'a> BuildFromExt<ImageTarget<'a>> for DatabaseOptions<'a> {
    type Error = ImageConfigError;

    fn build_from(input: ImageTarget<'a>) -> Result<Self, Self::Error> {
        let image = ImageOptions::try_from(input)?;
        Ok(DatabaseOptions { image, ..Self::default() })
    }
}
