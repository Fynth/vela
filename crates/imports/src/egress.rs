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
    #[cfg(test)]
    redirect_fixture: Option<(String, SocketAddr)>,
}

impl ControlledHttp {
    /// Validate both URL syntax and the complete resolved destination set without fetching content.
    pub async fn validate(&self, input: &str) -> Result<Url, ImportError> {
        let url = parse_url(input)?;
        self.resolve(&url).await?;
        Ok(url)
    }

    /// Requests do not inherit proxy settings, cookies, credentials, compression, or redirects.
    pub async fn request(&self, url: Url, method: Method) -> Result<Response, ImportError> {
        self.request_inner(url, method, false).await
    }

    /// Catalog identity may redirect only within its original HTTPS origin.
    pub async fn request_same_origin(
        &self,
        url: Url,
        method: Method,
    ) -> Result<Response, ImportError> {
        self.request_inner(url, method, true).await
    }
    async fn request_inner(
        &self,
        mut url: Url,
        method: Method,
        same_origin: bool,
    ) -> Result<Response, ImportError> {
        let origin = url.origin();
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
            if same_origin && next.origin() != origin {
                return Err(ImportError::Policy(
                    "Catalog redirects must keep the same origin",
                ));
            }
            url = next;
        }
        unreachable!("bounded redirect loop returns at its final hop")
    }

    async fn resolve(&self, url: &Url) -> Result<Vec<SocketAddr>, ImportError> {
        #[cfg(test)]
        for (host, address) in [self.fixture.as_ref(), self.redirect_fixture.as_ref()]
            .into_iter()
            .flatten()
        {
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
            redirect_fixture: None,
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
        Host::Domain(host) => {
            static DNS_SLOTS: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> =
                std::sync::OnceLock::new();
            let slots = DNS_SLOTS
                .get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(4)))
                .clone();
            let host = host.to_owned();
            dns_lookup_with(slots, move || {
                std::net::ToSocketAddrs::to_socket_addrs(&(host.as_str(), port))
            })
            .await?
        }
    };
    validate_addresses(&addresses)?;
    addresses.sort_unstable();
    addresses.dedup();
    Ok(addresses)
}

/// System resolution is blocking and cannot be canceled after it starts. The
/// global admission permit therefore belongs to the blocking closure, including
/// after a request deadline drops its JoinHandle. Waiting and actual resolution
/// share one deadline; a canceled waiter never starts another resolver job.
async fn dns_lookup_with<F, I>(
    slots: std::sync::Arc<tokio::sync::Semaphore>,
    lookup: F,
) -> Result<Vec<SocketAddr>, ImportError>
where
    F: FnOnce() -> std::io::Result<I> + Send + 'static,
    I: Iterator<Item = SocketAddr>,
{
    tokio::time::timeout(Duration::from_secs(10), async move {
        let permit = slots
            .acquire_owned()
            .await
            .map_err(|_| ImportError::Transient("DNS lookup unavailable"))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            lookup().map(|addresses| addresses.take(MAX_DNS_ADDRESSES + 1).collect())
        })
        .await
        .map_err(|_| ImportError::Transient("DNS lookup failed"))?
        .map_err(|_| ImportError::Transient("DNS lookup failed"))
    })
    .await
    .map_err(|_| ImportError::Transient("DNS lookup timed out"))?
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
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::mpsc,
        task::JoinHandle,
        time::timeout,
    };

    const REDIRECT_HOST: &str = "redirect.fixture.vela.example";
    const OTHER_HOST: &str = "other.fixture.vela.example";

    struct RedirectFixture {
        address: SocketAddr,
        connections: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<String>>>,
        dials: mpsc::Receiver<()>,
        task: JoinHandle<()>,
    }

    impl RedirectFixture {
        async fn new(respond: impl Fn(&str) -> String + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let connections = Arc::new(AtomicUsize::new(0));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let (dial_tx, dials) = mpsc::channel(16);
            let connection_count = connections.clone();
            let request_log = requests.clone();
            let task = tokio::spawn(async move {
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    connection_count.fetch_add(1, Ordering::SeqCst);
                    // Observe accepted TCP connections before reading any HTTP or TLS bytes.
                    dial_tx.try_send(()).unwrap();
                    let mut request = Vec::new();
                    timeout(Duration::from_secs(2), async {
                        let mut buffer = [0_u8; 512];
                        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                            let read = stream.read(&mut buffer).await.unwrap();
                            assert!(read > 0, "fixture request ended before its headers");
                            request.extend_from_slice(&buffer[..read]);
                            assert!(request.len() <= 8192, "fixture request exceeded its bound");
                        }
                    })
                    .await
                    .expect("fixture request headers timed out");
                    let request = String::from_utf8(request).unwrap();
                    let line = request.lines().next().unwrap().to_owned();
                    request_log.lock().unwrap().push(line.clone());
                    let response = respond(&line);
                    timeout(
                        Duration::from_secs(2),
                        stream.write_all(response.as_bytes()),
                    )
                    .await
                    .expect("fixture response timed out")
                    .unwrap();
                }
            });
            Self {
                address,
                connections,
                requests,
                dials,
                task,
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Drop for RedirectFixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn redirect(location: &str) -> String {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
    }

    fn catalog_response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[tokio::test]
    async fn same_origin_follows_relative_redirect_and_returns_actual_response() {
        let fixture = RedirectFixture::new(|request| match request {
            "GET /catalog/start HTTP/1.1" => redirect("../final?limit=20#discarded"),
            "GET /final?limit=20 HTTP/1.1" => catalog_response(r#"{"tracks":[]}"#),
            other => panic!("unexpected request: {other}"),
        })
        .await;
        let http = ControlledHttp::for_fixture(REDIRECT_HOST, fixture.address);
        let response = timeout(
            Duration::from_secs(3),
            http.request_same_origin(
                Url::parse(&format!("http://{REDIRECT_HOST}/catalog/start")).unwrap(),
                Method::GET,
            ),
        )
        .await
        .expect("same-origin fixture request timed out")
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.url().as_str(),
            format!("http://{REDIRECT_HOST}/final?limit=20")
        );
        assert_eq!(response.text().await.unwrap(), r#"{"tracks":[]}"#);
        assert_eq!(
            fixture.requests(),
            [
                "GET /catalog/start HTTP/1.1",
                "GET /final?limit=20 HTTP/1.1"
            ]
        );
        assert_eq!(fixture.connections.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn same_origin_rejects_other_host_before_any_request_or_tcp_dial() {
        let mut other = RedirectFixture::new(|request| {
            assert_eq!(request, "GET /catalog HTTP/1.1");
            catalog_response(r#"{"reachable":true}"#)
        })
        .await;
        let fixture = RedirectFixture::new(|request| {
            assert_eq!(request, "GET /start HTTP/1.1");
            redirect(&format!("http://{OTHER_HOST}/catalog"))
        })
        .await;
        let mut http = ControlledHttp::for_fixture(REDIRECT_HOST, fixture.address);
        http.redirect_fixture = Some((OTHER_HOST.to_owned(), other.address));
        let start = Url::parse(&format!("http://{REDIRECT_HOST}/start")).unwrap();

        // The ordinary import policy reaches the second isolated listener. This
        // proves that the rejection below comes from origin pinning, not DNS,
        // a dead listener, a blocked port, or the private-address syntax policy.
        let control = timeout(
            Duration::from_secs(3),
            http.request(start.clone(), Method::GET),
        )
        .await
        .expect("positive-control redirect timed out")
        .unwrap();
        assert_eq!(control.status(), StatusCode::OK);
        assert_eq!(control.url().host_str(), Some(OTHER_HOST));
        assert_eq!(control.text().await.unwrap(), r#"{"reachable":true}"#);
        assert_eq!(other.dials.recv().await, Some(()));

        let rejected = timeout(
            Duration::from_secs(3),
            http.request_same_origin(start, Method::GET),
        )
        .await
        .expect("cross-origin rejection timed out");
        assert!(matches!(
            rejected,
            Err(ImportError::Policy(
                "Catalog redirects must keep the same origin"
            ))
        ));
        assert!(
            timeout(Duration::from_millis(100), other.dials.recv())
                .await
                .is_err(),
            "cross-origin request dialed the second listener"
        );
        assert_eq!(other.connections.load(Ordering::SeqCst), 1);
        assert_eq!(other.requests(), ["GET /catalog HTTP/1.1"]);
        assert_eq!(fixture.connections.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn same_origin_rejects_scheme_change_before_a_tls_dial() {
        let mut fixture = RedirectFixture::new(|request| {
            assert_eq!(request, "GET /start HTTP/1.1");
            redirect(&format!("https://{REDIRECT_HOST}/catalog"))
        })
        .await;
        let http = ControlledHttp::for_fixture(REDIRECT_HOST, fixture.address);
        let rejected = timeout(
            Duration::from_secs(3),
            http.request_same_origin(
                Url::parse(&format!("http://{REDIRECT_HOST}/start")).unwrap(),
                Method::GET,
            ),
        )
        .await
        .expect("scheme-change rejection timed out");
        assert!(matches!(
            rejected,
            Err(ImportError::Policy(
                "Catalog redirects must keep the same origin"
            ))
        ));
        assert_eq!(fixture.dials.recv().await, Some(()));
        assert!(
            timeout(Duration::from_millis(100), fixture.dials.recv())
                .await
                .is_err(),
            "scheme-changing redirect initiated a TLS dial"
        );
        assert_eq!(fixture.connections.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.requests(), ["GET /start HTTP/1.1"]);
    }

    #[tokio::test]
    async fn same_origin_redirect_loop_stops_after_five_redirects() {
        let fixture = RedirectFixture::new(|request| {
            assert_eq!(request, "GET /loop?again=1 HTTP/1.1");
            redirect("/loop?again=1")
        })
        .await;
        let http = ControlledHttp::for_fixture(REDIRECT_HOST, fixture.address);
        let rejected = timeout(
            Duration::from_secs(3),
            http.request_same_origin(
                Url::parse(&format!("http://{REDIRECT_HOST}/loop?again=1")).unwrap(),
                Method::GET,
            ),
        )
        .await
        .expect("redirect-loop bound timed out");
        assert!(matches!(
            rejected,
            Err(ImportError::Policy("Too many redirects"))
        ));
        // Initial request + five followed redirects; the sixth response cannot
        // trigger another request, even though this listener remains available.
        assert_eq!(fixture.connections.load(Ordering::SeqCst), 6);
        assert_eq!(fixture.requests().len(), 6);
    }

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

#[cfg(test)]
mod dns_tests {
    use super::*;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    };
    use tokio::sync::Semaphore;

    // Always unblock the simulated resolver jobs, including on an assertion
    // failure. Their bounded receive deadline is a second cleanup safeguard.
    #[derive(Default)]
    struct DnsRelease(Vec<mpsc::Sender<()>>);

    impl DnsRelease {
        fn release(&mut self) {
            for sender in self.0.drain(..) {
                let _ = sender.send(());
            }
        }
    }

    impl Drop for DnsRelease {
        fn drop(&mut self) {
            self.release();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dns_cancellation_retains_running_worker_permits_and_prevents_a_fifth_spawn() {
        let slots = Arc::new(Semaphore::new(4));
        let entry = Arc::new(Barrier::new(4));
        let (started, mut ready) = tokio::sync::mpsc::channel(4);
        let mut release = DnsRelease::default();
        let mut requests = Vec::new();
        let address: SocketAddr = "8.8.8.8:443".parse().unwrap();
        for _ in 0..4 {
            let (sender, receiver) = mpsc::channel();
            release.0.push(sender);
            let started = started.clone();
            let entry = entry.clone();
            requests.push(tokio::spawn(dns_lookup_with(slots.clone(), move || {
                // All four actual blocking closures must enter before the
                // request futures are canceled, rather than merely queuing.
                entry.wait();
                started.blocking_send(()).unwrap();
                receiver
                    .recv_timeout(Duration::from_secs(10))
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))?;
                Ok(std::iter::once(address))
            })));
        }
        drop(started);
        tokio::time::timeout(Duration::from_secs(5), async {
            for _ in 0..4 {
                ready.recv().await.unwrap();
            }
        })
        .await
        .expect("the four injected blocking resolvers must start");
        let mut canceled = true;
        for request in requests {
            request.abort();
            canceled &= request.await.unwrap_err().is_cancelled();
        }
        let permits_after_cancel = slots.available_permits();
        let fifth_started = Arc::new(AtomicBool::new(false));
        let fifth_flag = fifth_started.clone();
        let mut fifth = tokio::spawn(dns_lookup_with(slots.clone(), move || {
            fifth_flag.store(true, Ordering::SeqCst);
            Ok(std::iter::once(address))
        }));
        let early = tokio::time::timeout(Duration::from_millis(50), &mut fifth).await;
        let fifth_waited = early.is_err();
        let fifth_ran_while_full = fifth_started.load(Ordering::SeqCst);
        release.release();
        let final_result = match early {
            Ok(result) => result,
            Err(_) => tokio::time::timeout(Duration::from_secs(5), fifth)
                .await
                .expect("the fifth resolver must proceed after real worker completion"),
        }
        .unwrap()
        .unwrap();
        // Wait for every detached original worker to release its actual slot.
        let returned =
            tokio::time::timeout(Duration::from_secs(5), slots.clone().acquire_many_owned(4))
                .await
                .unwrap()
                .unwrap();
        drop(returned);
        assert!(canceled);
        assert_eq!(permits_after_cancel, 0);
        assert!(fifth_waited);
        assert!(!fifth_ran_while_full);
        assert!(fifth_started.load(Ordering::SeqCst));
        assert_eq!(final_result, vec![address]);
        assert_eq!(slots.available_permits(), 4);
    }

    #[tokio::test]
    async fn dns_lookup_consumes_only_the_bounded_address_prefix() {
        let consumed = Arc::new(AtomicUsize::new(0));
        let observed = consumed.clone();
        let address: SocketAddr = "8.8.8.8:443".parse().unwrap();
        let addresses = dns_lookup_with(Arc::new(Semaphore::new(4)), move || {
            Ok(std::iter::repeat(address).inspect(move |_| {
                observed.fetch_add(1, Ordering::SeqCst);
            }))
        })
        .await
        .unwrap();
        assert_eq!(consumed.load(Ordering::SeqCst), MAX_DNS_ADDRESSES + 1);
        assert_eq!(addresses.len(), MAX_DNS_ADDRESSES + 1);
        assert!(validate_addresses(&addresses).is_err());
    }

    #[tokio::test]
    async fn dns_literal_destinations_remain_direct_and_policy_checked() {
        assert_eq!(
            resolve(&Url::parse("https://8.8.8.8/").unwrap())
                .await
                .unwrap(),
            vec!["8.8.8.8:443".parse::<SocketAddr>().unwrap()]
        );
        assert!(
            resolve(&Url::parse("https://127.0.0.1/").unwrap())
                .await
                .is_err()
        );
    }
}
