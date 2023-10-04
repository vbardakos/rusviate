pub mod config;
pub mod errors;

use std::{
    net::{TcpListener, SocketAddr, IpAddr}
};
use std::path::{PathBuf};
use sha2::{Sha256, Digest};
use tokio::net::TcpSocket;
use error_stack::{Report, ResultExt};
use sha2::digest::FixedOutput;
use crate::database::errors::DatabaseError;

#[derive(Debug, Default)]
pub struct Database<'a> {
    options: config::DatabaseOptions<'a>
}

trait TcpBaseOpsExt {
    type Error;

    fn available_port(host: IpAddr) -> std::result::Result<u16, Self::Error>;
    fn socket(&self) -> std::result::Result<TcpSocket, Self::Error>;
    fn socket_listens(&self, socket: TcpSocket) -> std::result::Result<bool, Self::Error>;
}

pub trait DatabaseBaseOpsExt {
    type Error;

    fn start(&self) -> Result<(), Self::Error>;
    fn stop(&self) -> Result<(), Self::Error>;
}

impl<'a> TcpBaseOpsExt for Database<'a> {
    type Error = Report<DatabaseError>;

    /// equivalent of get_random_port -- wtf is it tho'?
    fn available_port(host: IpAddr) -> Result<u16, Self::Error> {
        let listener = TcpListener::bind((host, 0))
            .change_context(DatabaseError::TcpError)
            .attach_printable(format!("Listener failed to bind host: {host:?}"))?;

        let addr = listener.local_addr()
            .change_context(DatabaseError::TcpError)
            .attach_printable("Listener failed to retrieve Address")?;

        Ok(addr.port())
    }

    /// retrieves the appropriate socket
    fn socket(&self) -> Result<TcpSocket, Self::Error> {
        let socket = match &self.options.socket_addr {
            SocketAddr::V4(_) => TcpSocket::new_v4(),
            SocketAddr::V6(_) => TcpSocket::new_v6(),
        };

        socket.change_context(DatabaseError::TcpError)
            .attach_printable("Unexpected Error during the Socket retrieval")
    }

    /// checks if socket can be bound
    fn socket_listens(&self, socket: TcpSocket) -> Result<bool, Self::Error> {
        match socket.bind(self.options.socket_addr) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => Ok(false),
            Err(e) => {
                Err(Report::new(DatabaseError::TcpError))
                    .attach_printable(format!("Unexpected Bind Error: {:?}", e.kind()))
            }
        }
    }
}



impl<'a> Database<'a> {
    pub fn new(options: config::DatabaseOptions<'a>) -> Self {
        Database { options }
    }

    /// retrieves executable endpoint
    fn binary_endpoint(&self) -> PathBuf {
        let mut bin_dir = self.options.binary_dir.to_path_buf();

        let version = &self.options.image.version;
        let sha256 = Sha256::digest(version);

        bin_dir.push(format!("weaviate-{version}-{:x}", sha256));
        bin_dir
    }

    /// builds executable
    fn build_binary(&self) -> error_stack::Result<(), DatabaseError> {
        if !self.binary_endpoint().exists() {
            match &self.options.image.target {
                config::ImageTarget::Local(file) => (),
                config::ImageTarget::Url(file) => self.download_binary()?,
            }
        }
        Ok(())
    }

    /// downloads executable
    fn download_binary(&self) -> error_stack::Result<(), DatabaseError> {
        Ok(())
    }
}
