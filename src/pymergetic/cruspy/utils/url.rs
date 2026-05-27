//! Generic URL parse / format — no scheme-specific semantics.
//!
//! Parsed values are [`Range`]s into one backing [`String`] so `scheme()`, `host()`, `path()`, …
//! are cheap slices without extra allocations.

use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    buf: String,
    scheme: Range<usize>,
    user: Option<Range<usize>>,
    host: Range<usize>,
    port: Option<u16>,
    path: Range<usize>,
    query: Option<Range<usize>>,
    fragment: Option<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid URL: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

#[inline]
fn slice<'a>(buf: &'a str, range: &Range<usize>) -> &'a str {
    &buf[range.start..range.end]
}

impl Url {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        s.parse()
    }

    pub fn builder() -> UrlBuilder {
        UrlBuilder::default()
    }

    pub fn as_str(&self) -> &str {
        &self.buf
    }

    pub fn scheme(&self) -> &str {
        slice(&self.buf, &self.scheme)
    }

    /// Userinfo before `@` in the authority (if any).
    pub fn user(&self) -> Option<&str> {
        self.user.as_ref().map(|r| slice(&self.buf, r))
    }

    pub fn host(&self) -> &str {
        slice(&self.buf, &self.host)
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn path(&self) -> &str {
        slice(&self.buf, &self.path)
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_ref().map(|r| slice(&self.buf, r))
    }

    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_ref().map(|r| slice(&self.buf, r))
    }

    pub fn is_file(&self) -> bool {
        self.scheme() == "file"
    }

    /// Path component when [`Self::is_file`] (empty path → `None`).
    pub fn file_path(&self) -> Option<&Path> {
        if !self.is_file() {
            return None;
        }
        let p = self.path();
        if p.is_empty() {
            None
        } else {
            Some(Path::new(p))
        }
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.buf)
    }
}

impl FromStr for Url {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let scheme_end = s
            .find(':')
            .ok_or_else(|| ParseError(format!("missing scheme in {s:?}")))?;
        if s.len() < scheme_end + 3 || &s[scheme_end..scheme_end + 3] != "://" {
            return Err(ParseError(format!("missing :// after scheme in {s:?}")));
        }

        let scheme = 0..scheme_end;
        let i = scheme_end + 3;

        let path_start = s[i..]
            .find(|c| c == '/' || c == '?' || c == '#')
            .map(|off| i + off)
            .unwrap_or(s.len());

        let authority = &s[i..path_start];
        let (user, host, port) = parse_authority(authority)?;

        let (path, query, fragment) = parse_path_query_fragment(&s[path_start..])?;

        let user_range = user.map(|u| range_in(s, u));
        let host_range = range_in(s, host);
        let path_range = range_in(s, path);
        let query_range = query.map(|q| range_in(s, q));
        let fragment_range = fragment.map(|fr| range_in(s, fr));

        Ok(Self {
            buf: s.to_owned(),
            scheme,
            user: user_range,
            host: host_range,
            port,
            path: path_range,
            query: query_range,
            fragment: fragment_range,
        })
    }
}

fn range_in(haystack: &str, part: &str) -> Range<usize> {
    let start = haystack
        .find(part)
        .unwrap_or_else(|| panic!("URL part not found in buffer"));
    start..start + part.len()
}

fn parse_authority(authority: &str) -> Result<(Option<&str>, &str, Option<u16>), ParseError> {
    let (user, host_port) = match authority.split_once('@') {
        Some((u, rest)) => (Some(u), rest),
        None => (None, authority),
    };
    if host_port.is_empty() && user.is_some() {
        return Ok((user, "", None));
    }
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => {
            let port = p
                .parse::<u16>()
                .map_err(|_| ParseError(format!("bad port {p:?}")))?;
            (h, Some(port))
        }
        _ => (host_port, None),
    };
    Ok((user, host, port))
}

fn parse_path_query_fragment(
    rest: &str,
) -> Result<(&str, Option<&str>, Option<&str>), ParseError> {
    if rest.is_empty() {
        return Ok(("", None, None));
    }
    let fragment_off = rest.find('#');
    let without_frag = match fragment_off {
        Some(i) => &rest[..i],
        None => rest,
    };
    let fragment = fragment_off.map(|i| &rest[i + 1..]);

    let query_off = without_frag.find('?');
    let (path, query) = match query_off {
        Some(i) => (&without_frag[..i], Some(&without_frag[i + 1..])),
        None => (without_frag, None),
    };
    Ok((path, query, fragment))
}

#[derive(Default)]
pub struct UrlBuilder {
    scheme: String,
    user: Option<String>,
    host: String,
    port: Option<u16>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl UrlBuilder {
    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into();
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn fragment(mut self, fragment: impl Into<String>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }

    pub fn build(self) -> Url {
        let mut buf = String::new();
        buf.push_str(&self.scheme);
        buf.push_str("://");
        if let Some(user) = &self.user {
            buf.push_str(user);
            buf.push('@');
        }
        buf.push_str(&self.host);
        if let Some(port) = self.port {
            buf.push(':');
            buf.push_str(&port.to_string());
        }
        if !self.path.is_empty() {
            if !self.path.starts_with('/') {
                buf.push('/');
            }
            buf.push_str(&self.path);
        }
        if let Some(query) = &self.query {
            buf.push('?');
            buf.push_str(query);
        }
        if let Some(fragment) = &self.fragment {
            buf.push('#');
            buf.push_str(fragment);
        }
        Url::parse(&buf).expect("built URL must parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scheme_host() {
        let url = Url::parse("ram://heap").unwrap();
        assert_eq!(url.scheme(), "ram");
        assert_eq!(url.host(), "heap");
        assert_eq!(url.path(), "");
        assert_eq!(url.to_string(), "ram://heap");
    }

    #[test]
    fn parse_file_path() {
        let url = Url::parse("file:///tmp/foo.bin").unwrap();
        assert!(url.is_file());
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), "/tmp/foo.bin");
        assert_eq!(url.file_path(), Some(Path::new("/tmp/foo.bin")));
    }

    #[test]
    fn ram_is_not_file() {
        let url = Url::parse("ram://heap").unwrap();
        assert!(!url.is_file());
        assert_eq!(url.file_path(), None);
    }

    #[test]
    fn builder_roundtrip() {
        let url = Url::builder()
            .scheme("shm")
            .host("session")
            .build();
        assert_eq!(url.to_string(), "shm://session");
        let again = Url::parse(url.as_str()).unwrap();
        assert_eq!(again.host(), "session");
    }
}
