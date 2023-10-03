pub mod config;
pub mod errors;

use std::net::{TcpListener, SocketAddr, IpAddr};
use tokio::net::TcpSocket;
use error_stack::{Report, ResultExt};
use crate::database::errors::DatabaseError;

#[derive(Debug)]
pub struct Database<'a> {
    listener: u16,
    options: config::DatabaseOptions<'a>
}

trait TcpBaseOpsExt {
    type Error;

    fn available_port(host: IpAddr) -> std::result::Result<u16, Self::Error>;
    fn socket(&self) -> std::result::Result<TcpSocket, Self::Error>;
    fn socket_listens(&self, socket: TcpSocket) -> std::result::Result<bool, Self::Error>;
}

impl<'a> TcpBaseOpsExt for Database<'a> {
    type Error = Report<DatabaseError>;

    /// equivalent of get_random_port
    fn available_port(host: IpAddr) -> std::result::Result<u16, Self::Error> {
        let listener = TcpListener::bind((host, 0))
            .change_context(DatabaseError::TcpError)
            .attach_printable(format!("Listener failed to bind host: {host:?}"))?;

        let addr = listener.local_addr()
            .change_context(DatabaseError::TcpError)
            .attach_printable("Listener failed to retrieve Address")?;

        Ok(addr.port())
    }

    /// retrieves the appropriate socket
    fn socket(&self) -> std::result::Result<TcpSocket, Self::Error> {
        let socket = match &self.options.socket_addr {
            SocketAddr::V4(_) => TcpSocket::new_v4(),
            SocketAddr::V6(_) => TcpSocket::new_v6(),
        };

        socket.change_context(DatabaseError::TcpError)
            .attach_printable("Unexpected Error during the Socket retrieval")
    }

    /// checks if socket can be bound
    fn socket_listens(&self, socket: TcpSocket) -> std::result::Result<bool, Self::Error> {
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


pub trait DatabaseBaseOpsExt {
    type Error;

    fn start(&self) -> std::result::Result<(), Self::Error>;
    fn stop(&self) -> std::result::Result<(), Self::Error>;
}
