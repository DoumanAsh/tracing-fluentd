use crate::MakeWriter;

use std::net::ToSocketAddrs;

impl MakeWriter for std::vec::IntoIter<std::net::SocketAddr> {
    type Writer = std::net::TcpStream;

    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        for addr in self.as_slice().iter() {
            match std::net::TcpStream::connect_timeout(addr, core::time::Duration::from_secs(1)) {
                Ok(socket) => return Ok(socket),
                Err(_) => continue,
            }
        }

        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd"))
    }
}

///Creates writer by resolving address from provided string.
impl MakeWriter for &'static str {
    type Writer = std::net::TcpStream;

    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        let addrs = self.to_socket_addrs()?;

        for addr in addrs.as_slice().iter() {
            match std::net::TcpStream::connect_timeout(addr, core::time::Duration::from_secs(1)) {
                Ok(socket) => return Ok(socket),
                Err(_) => continue,
            }
        }

        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd"))
    }
}

///Creates writer by resolving address from provided string and port.
impl MakeWriter for (&'static str, u16) {
    type Writer = std::net::TcpStream;

    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        let addrs = self.to_socket_addrs()?;

        for addr in addrs.as_slice().iter() {
            match std::net::TcpStream::connect_timeout(addr, core::time::Duration::from_secs(1)) {
                Ok(socket) => return Ok(socket),
                Err(_) => continue,
            }
        }

        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd"))
    }
}

impl MakeWriter for std::net::SocketAddr {
    type Writer = std::net::TcpStream;

    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        match std::net::TcpStream::connect_timeout(self, core::time::Duration::from_secs(1)) {
            Ok(socket) => Ok(socket),
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd")),
        }
    }
}

impl MakeWriter for [std::net::SocketAddr; 1] {
    type Writer = std::net::TcpStream;

    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        match std::net::TcpStream::connect_timeout(&self[0], core::time::Duration::from_secs(1)) {
            Ok(socket) => Ok(socket),
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd")),
        }
    }
}

//While we can use generics, it doesn't really make sense to store addresses in such big arrays.
macro_rules! impl_for_socket_addr_array {
    ($($idx:literal),+) => {

        $(
            impl MakeWriter for [std::net::SocketAddr; $idx] {
                type Writer = std::net::TcpStream;

                #[inline(always)]
                fn make(&self) -> std::io::Result<Self::Writer> {
                    for addr in self {
                        match std::net::TcpStream::connect_timeout(addr, core::time::Duration::from_secs(1)) {
                            Ok(socket) => return Ok(socket),
                            Err(_) => continue,
                        }
                    }

                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cannot connect to fluentd"))
                }
            }
        )+
    }
}

impl_for_socket_addr_array!(2,3,4,5,6,7,8,9,10,11,12);
