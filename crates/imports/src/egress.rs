//! Public-Internet-only HTTP transport. Every hop receives a newly pinned DNS snapshot.
use crate::ImportError;
use reqwest::{
    Method, Response, StatusCode,
    header::{ACCEPT_ENCODING, LOCATION},
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
use url::{Host, Url};

const MAX_REDIRECTS: usize = 5;
const MAX_URL_BYTES: usize = 4096;
const MAX_DNS_ADDRESSES: usize = 16;

/// Shared egress policy for import adapters. There is deliberately no production allow-private flag.
#[derive(Clone, Default)]
pub struct ControlledHttp {
    // Only unit-test builds can pin a specifically named fixture to a loopback test listener.
    #[cfg(test)]
    fixture: Option<(String, SocketAddr)>,
}

impl ControlledHttp {
    /// Validate both URL syntax and the complete resolved destination set without fetching content.
    pub async fn validate(&self, input: &str) -> Result<Url, ImportError> {
        let url = parse_url(input)?;
        self.resolve(&url).await?;
        Ok(url)
    }

    /// Requests do not inherit proxy settings, cookies, credentials, compression, or redirects.
    pub async fn request(&self, mut url: Url, method: Method) -> Result<Response, ImportError> {
        for hop in 0..=MAX_REDIRECTS {
            let addresses = self.resolve(&url).await?;
            let host = url
                .host_str()
                .ok_or(ImportError::Policy("URL needs a host"))?;
            let client = reqwest::Client::builder()
                .no_proxy()
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .redirect(reqwest::redirect::Policy::none())
                .referer(false)
                .connect_timeout(Duration::from_secs(10))
                .read_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(900))
                .pool_max_idle_per_host(0)
                .user_agent(concat!("VelaMusic/", env!("CARGO_PKG_VERSION")))
                .resolve_to_addrs(host, &addresses)
                .build()
                .map_err(|_| ImportError::Policy("HTTP client initialization failed"))?;
            let response = client
                .request(method.clone(), url.clone())
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(network_error)?;
            if !matches!(
                response.status(),
                StatusCode::MOVED_PERMANENTLY
                    | StatusCode::FOUND
                    | StatusCode::SEE_OTHER
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::PERMANENT_REDIRECT
            ) {
                return Ok(response);
            }
            if hop == MAX_REDIRECTS {
                return Err(ImportError::Policy("Too many redirects"));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .filter(|v| v.len() <= MAX_URL_BYTES)
                .ok_or(ImportError::Policy("Invalid redirect location"))?;
            let next = url
                .join(location)
                .map_err(|_| ImportError::Policy("Invalid redirect URL"))?;
            let next = parse_url(next.as_str())?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err(ImportError::Policy("HTTPS redirects must stay on HTTPS"));
            }
            url = next;
        }
        unreachable!("bounded redirect loop returns at its final hop")
    }

    async fn resolve(&self, url: &Url) -> Result<Vec<SocketAddr>, ImportError> {
        #[cfg(test)]
        if let Some((host, address)) = &self.fixture {
            let checked = parse_url(url.as_str())?;
            if checked.host_str() == Some(host) {
                return Ok(vec![*address]);
            }
        }
        resolve(url).await
    }

    #[cfg(test)]
    pub(crate) fn for_fixture(host: &str, address: SocketAddr) -> Self {
        Self {
            fixture: Some((host.to_string(), address)),
        }
    }
}

pub(crate) fn network_error(error: reqwest::Error) -> ImportError {
    if error.is_timeout() || error.is_connect() || error.is_body() {
        ImportError::Transient("Remote connection failed or timed out")
    } else {
        ImportError::Policy("Remote server returned an invalid HTTP response")
    }
}

pub(crate) fn parse_url(input: &str) -> Result<Url, ImportError> {
    if input.is_empty() || input.len() > MAX_URL_BYTES || input.chars().any(char::is_control) {
        return Err(ImportError::Policy("Invalid or overly long URL"));
    }
    // Even empty userinfo is forbidden; URL normalization otherwise discards it.
    if input.split_once("://").is_some_and(|(_, rest)| {
        rest.split(['/', '?', '#'])
            .next()
            .is_some_and(|authority| authority.contains('@'))
    }) {
        return Err(ImportError::Policy("Credentials in URLs are not supported"));
    }
    let mut url = Url::parse(input).map_err(|_| ImportError::Policy("Invalid URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ImportError::Policy(
            "Only HTTP and HTTPS audio URLs are supported",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ImportError::Policy("Credentials in URLs are not supported"));
    }
    if !matches!(url.port_or_known_default(), Some(80 | 443)) {
        return Err(ImportError::Policy(
            "Only destination ports 80 and 443 are permitted",
        ));
    }
    match url.host().ok_or(ImportError::Policy("URL needs a host"))? {
        Host::Ipv4(ip) if !public_address(IpAddr::V4(ip)) => {
            return Err(ImportError::Policy(
                "Private or special-purpose destinations are not permitted",
            ));
        }
        Host::Ipv6(ip) if !public_address(IpAddr::V6(ip)) => {
            return Err(ImportError::Policy(
                "Private or special-purpose destinations are not permitted",
            ));
        }
        Host::Domain(host) => {
            let normalized = host.trim_end_matches('.').to_ascii_lowercase();
            if !normalized.contains('.')
                || [
                    "localhost",
                    "local",
                    "internal",
                    "home",
                    "test",
                    "invalid",
                    "onion",
                ]
                .iter()
                .any(|suffix| normalized == *suffix || normalized.ends_with(&format!(".{suffix}")))
            {
                return Err(ImportError::Policy(
                    "Local or reserved hostnames are not permitted",
                ));
            }
        }
        _ => {}
    }
    url.set_fragment(None);
    Ok(url)
}

async fn resolve(url: &Url) -> Result<Vec<SocketAddr>, ImportError> {
    // Call syntax validation here as adapters can construct URLs independently.
    let checked = parse_url(url.as_str())?;
    let port = checked
        .port_or_known_default()
        .ok_or(ImportError::Policy("Invalid port"))?;
    let mut addresses = match checked
        .host()
        .ok_or(ImportError::Policy("URL needs a host"))?
    {
        Host::Ipv4(ip) => vec![SocketAddr::new(ip.into(), port)],
        Host::Ipv6(ip) => vec![SocketAddr::new(ip.into(), port)],
        Host::Domain(host) => tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::lookup_host((host, port)),
        )
        .await
        .map_err(|_| ImportError::Transient("DNS lookup timed out"))?
        .map_err(|_| ImportError::Transient("DNS lookup failed"))?
        .take(MAX_DNS_ADDRESSES + 1)
        .collect(),
    };
    validate_addresses(&addresses)?;
    addresses.sort_unstable();
    addresses.dedup();
    Ok(addresses)
}

fn validate_addresses(addresses: &[SocketAddr]) -> Result<(), ImportError> {
    if addresses.is_empty() || addresses.len() > MAX_DNS_ADDRESSES {
        return Err(ImportError::Policy(
            "DNS returned an empty or overly large address set",
        ));
    }
    if addresses.iter().any(|addr| !public_address(addr.ip())) {
        return Err(ImportError::Policy(
            "Private or special-purpose destinations are not permitted",
        ));
    }
    Ok(())
}

fn v4_prefix(ip: Ipv4Addr, base: Ipv4Addr, bits: u32) -> bool {
    let mask = u32::MAX << (32 - bits);
    u32::from(ip) & mask == u32::from(base) & mask
}

fn v6_prefix(ip: Ipv6Addr, base: Ipv6Addr, bits: u32) -> bool {
    let mask = u128::MAX << (128 - bits);
    u128::from(ip) & mask == u128::from(base) & mask
}

/// Conservative public-unicast policy based on the IANA special-purpose registries.
/// Native IPv6 global unicast is supported; transition, mapped and special-purpose ranges are not.
pub fn public_address(ip: IpAddr) -> bool {
    match ip {
        // Azure's host-service virtual address is public-looking but local to its platform.
        IpAddr::V4(ip) => {
            ip != Ipv4Addr::new(168, 63, 129, 16)
                && ![
                    (Ipv4Addr::new(0, 0, 0, 0), 8),
                    (Ipv4Addr::new(10, 0, 0, 0), 8),
                    (Ipv4Addr::new(100, 64, 0, 0), 10),
                    (Ipv4Addr::new(127, 0, 0, 0), 8),
                    (Ipv4Addr::new(169, 254, 0, 0), 16),
                    (Ipv4Addr::new(172, 16, 0, 0), 12),
                    (Ipv4Addr::new(192, 0, 0, 0), 24),
                    (Ipv4Addr::new(192, 0, 2, 0), 24),
                    (Ipv4Addr::new(192, 88, 99, 0), 24),
                    (Ipv4Addr::new(192, 168, 0, 0), 16),
                    (Ipv4Addr::new(198, 18, 0, 0), 15),
                    (Ipv4Addr::new(198, 51, 100, 0), 24),
                    (Ipv4Addr::new(203, 0, 113, 0), 24),
                    (Ipv4Addr::new(224, 0, 0, 0), 3),
                ]
                .iter()
                .any(|(base, bits)| v4_prefix(ip, *base, *bits))
        }
        IpAddr::V6(ip) => {
            v6_prefix(ip, Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0), 3)
                && ![
                    (Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23), // Teredo and special protocol assignments.
                    (Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0), 32),
                    (Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16), // 6to4 embeds arbitrary IPv4 destinations.
                    (Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0), 20), // Documentation.
                ]
                .iter()
                .any(|(base, bits)| v6_prefix(ip, *base, *bits))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_adversarial_urls() {
        for input in [
            "file:///etc/passwd",
            "ftp://example.com/music.mp3",
            "http://localhost/x",
            "http://localhost./x",
            "http://server.local/x",
            "http://internal/x",
            "http://user:pass@example.com/x",
            "http://user@example.com/x",
            "http://@example.com/x",
            "http://example.com:8080/x",
            "http://127.1/x",
            "http://2130706433/x",
            "http://0x7f000001/x",
            "http://0177.0.0.1/x",
            "http://169.254.169.254/latest/meta-data",
            "http://[::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://[64:ff9b::a00:1]/x",
            "https://example.com/\nfile",
        ] {
            assert!(parse_url(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn rejects_special_addresses_including_cloud_metadata_and_transitions() {
        for ip in [
            "0.0.0.0",
            "10.255.255.255",
            "100.64.0.1",
            "100.100.100.200",
            "127.0.0.1",
            "169.254.169.254",
            "168.63.129.16",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:8.8.8.8",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "100::1",
            "2001::1",
            "2001:db8::1",
            "2002:7f00:1::",
            "3fff::1",
            "5f00::1",
            "fc00::1",
            "fd00:ec2::254",
            "fe80::1",
            "ff02::1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "accepted {ip}");
        }
    }

    #[test]
    fn allows_real_global_addresses_and_rejects_mixed_dns() {
        for ip in [
            "8.8.8.8",
            "1.1.1.1",
            "100.63.255.255",
            "100.128.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "192.169.1.1",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(public_address(ip.parse().unwrap()), "rejected {ip}");
        }
        assert!(
            validate_addresses(&[
                "8.8.8.8:443".parse().unwrap(),
                "127.0.0.1:443".parse().unwrap()
            ])
            .is_err()
        );
        assert!(validate_addresses(&[]).is_err());
        assert!(validate_addresses(&vec!["8.8.8.8:443".parse().unwrap(); 17]).is_err());
        assert_eq!(
            parse_url("https://example.com/a.flac#fragment")
                .unwrap()
                .as_str(),
            "https://example.com/a.flac"
        );
    }
}
