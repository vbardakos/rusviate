use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use crate::errors::DbBuilderErrors;
use error_stack::{Report, Result, ResultExt};
use std::env::consts::{ARCH, OS};
use simple_home_dir::expand_tilde;


const DATA: &str = "~/.local/share/weaviate";
const ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8079);
const BIN: &str = "~/.cache/weaviate-embedded/";
const GIT: &str = "https://github.com/weaviate/weaviate/releases/download";


#[derive(Debug, Clone)]
pub enum Image<'a> {
    Local(Cow<'a, Path>),
    Url(Cow<'a, str>),
}

/// Database Available Systems (Windows is not supported)
#[derive(Debug, Clone)]
pub enum System {
    MacOS,
    LinuxX86_64,
    LinuxAArch64,
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
    system: Option<System>,
    target: Option<Image<'a>>,
    conn_retries: Option<(usize, usize)>,
    extras: Option<HashMap<&'a str, &'a str>>
}

#[derive(Debug)]
pub struct EmbeddedDatabase<'a> {
    bin: Box<Path>,
    addr: SocketAddr,  // might be TcpSocket instead
    version: String,
    retries: (usize, usize),
    extras: HashMap<&'a str, &'a str>
}
