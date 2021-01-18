//! Iron's HTTP Request representation and associated methods.

use std::fmt::{self, Debug};
use std::io::{self, Read};
use std::net::SocketAddr;

use hyper::http::h1::HttpReader;
use hyper::net::NetworkStream;
use hyper::uri::RequestUri::{AbsolutePath, AbsoluteUri};
use hyper::version::HttpVersion;

use hyper::method::Method;
use plugin::Extensible;
use typemap::TypeMap;

use hyper::buffer;
pub use hyper::server::request::Request as HttpRequest;

#[cfg(test)]
use std::net::ToSocketAddrs;

pub use crate::request::url::Url;

use crate::iron::Protocol;
use crate::{headers, modifier::Set, Headers, Plugin};

mod url;

/// The `Request` given to all `Middleware`.
///
/// Stores all the properties of the client's request plus
/// an `TypeMap` for data communication between middleware.
#[non_exhaustive]
pub struct Request<'a, 'b: 'a> {
    /// The requested URL.
    pub url: Url,

    /// The originating address of the request.
    pub remote_addr: SocketAddr,

    /// The local address of the request.
    pub local_addr: SocketAddr,

    /// The request headers.
    pub headers: Headers,

    /// The request body as a reader.
    pub body: Body<'a, 'b>,

    /// The request method.
    pub method: Method,

    /// Extensible storage for data passed between middleware.
    pub extensions: TypeMap,

    /// The version of the HTTP protocol used.
    pub version: HttpVersion,
}

impl<'a, 'b> Debug for Request<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Request {{")?;
        writeln!(f, "    url: {:?}", self.url)?;
        writeln!(f, "    method: {:?}", self.method)?;
        writeln!(f, "    remote_addr: {:?}", self.remote_addr)?;
        writeln!(f, "    local_addr: {:?}", self.local_addr)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl<'a, 'b> Request<'a, 'b> {
    /// Create a request from an HttpRequest.
    ///
    /// This constructor consumes the HttpRequest.
    pub fn from_http(
        req: HttpRequest<'a, 'b>,
        local_addr: SocketAddr,
        protocol: &Protocol,
    ) -> Result<Request<'a, 'b>, String> {
        let (addr, method, headers, uri, version, reader) = req.deconstruct();

        let url = match uri {
            AbsoluteUri(ref url) => match Url::from_generic_url(url.clone()) {
                Ok(url) => url,
                Err(e) => return Err(e),
            },

            AbsolutePath(ref path) => {
                let url_string = match (version, headers.get::<headers::Host>()) {
                    (_, Some(host)) => {
                        // Attempt to prepend the Host header (mandatory in HTTP/1.1)
                        if let Some(port) = host.port {
                            format!("{}://{}:{}{}", protocol.name(), host.hostname, port, path)
                        } else {
                            format!("{}://{}{}", protocol.name(), host.hostname, path)
                        }
                    }
                    (v, None) if v < HttpVersion::Http11 => {
                        // Attempt to use the local address? (host header is not required in HTTP/1.0).
                        match local_addr {
                            SocketAddr::V4(addr4) => format!(
                                "{}://{}:{}{}",
                                protocol.name(),
                                addr4.ip(),
                                local_addr.port(),
                                path
                            ),
                            SocketAddr::V6(addr6) => format!(
                                "{}://[{}]:{}{}",
                                protocol.name(),
                                addr6.ip(),
                                local_addr.port(),
                                path
                            ),
                        }
                    }
                    (_, None) => return Err("No host specified in request".into()),
                };

                match Url::parse(&url_string) {
                    Ok(url) => url,
                    Err(e) => return Err(format!("Couldn't parse requested URL: {}", e)),
                }
            }
            _ => return Err("Unsupported request URI".into()),
        };

        Ok(Request {
            url,
            remote_addr: addr,
            local_addr,
            headers,
            body: Body::new(reader),
            method,
            extensions: TypeMap::new(),
            version,
        })
    }

    #[cfg(test)]
    pub fn stub() -> Request<'a, 'b> {
        Request {
            url: Url::parse("http://www.rust-lang.org").unwrap(),
            remote_addr: "localhost:3000".to_socket_addrs().unwrap().next().unwrap(),
            local_addr: "localhost:3000".to_socket_addrs().unwrap().next().unwrap(),
            headers: Headers::new(),
            body: unsafe { std::mem::MaybeUninit::uninit().assume_init() }, // FIXME(reem): Ugh
            method: Method::Get,
            extensions: TypeMap::new(),
            version: HttpVersion::Http11,
        }
    }
}

/// The body of an Iron request,
pub struct Body<'a, 'b: 'a>(HttpReader<&'a mut buffer::BufReader<&'b mut NetworkStream>>);

impl<'a, 'b> Body<'a, 'b> {
    /// Create a new reader for use in an Iron request from a hyper HttpReader.
    pub fn new(
        reader: HttpReader<&'a mut buffer::BufReader<&'b mut NetworkStream>>,
    ) -> Body<'a, 'b> {
        Body(reader)
    }
}

impl<'a, 'b> Read for Body<'a, 'b> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

// Allow plugins to attach to requests.
impl<'a, 'b> Extensible for Request<'a, 'b> {
    fn extensions(&self) -> &TypeMap {
        &self.extensions
    }

    fn extensions_mut(&mut self) -> &mut TypeMap {
        &mut self.extensions
    }
}

impl<'a, 'b> Plugin for Request<'a, 'b> {}
impl<'a, 'b> Set for Request<'a, 'b> {}
