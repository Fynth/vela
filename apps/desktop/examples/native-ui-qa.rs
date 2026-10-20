//! Opt-in real-window integration driver. Never compiled into the shipping app.
use dioxus::{desktop::Config, prelude::*};
use serde_json::{Value, json};
use std::path::PathBuf;

#[allow(non_snake_case)]
fn Harness() -> Element {
    use_future(|| async {
        let report =
            PathBuf::from(std::env::var_os("VELA_NATIVE_QA_REPORT").expect("fixture report path"));
        let mut eval = document::eval(include_str!("native-ui-qa.js"));
        eval.send(json!({
            "decorations": dioxus::desktop::window().is_decorated(),
            "windowOnly": std::env::var("VELA_NATIVE_QA_WINDOW_ONLY").as_deref() == Ok("1"),
            "origin": std::env::var("VELA_DESKTOP_SERVER_URL").expect("isolated fixture origin"),
            "username": std::env::var("VELA_NATIVE_QA_USERNAME").expect("fixture username"),
            "password": std::env::var("VELA_NATIVE_QA_PASSWORD").expect("fixture password"),
            "closeWhilePlaying": std::env::var("VELA_NATIVE_QA_CLOSE_PLAYING").as_deref() == Ok("1"),
            "peerOrigin": std::env::var("VELA_NATIVE_QA_PEER_ORIGIN").unwrap_or_default(),
            "peer2Origin": std::env::var("VELA_NATIVE_QA_PEER2_ORIGIN").unwrap_or_default(),
            "cachedOrigin": std::env::var("VELA_NATIVE_QA_CACHED_ORIGIN").unwrap_or_default(),
            "sessionPhase": std::env::var("VELA_NATIVE_QA_SESSION_PHASE").unwrap_or_default(),
            "testSession": std::env::var("VELA_NATIVE_QA_TEST_SESSION").as_deref() == Ok("1"),
            "windowWidth": qa_dimension("VELA_NATIVE_QA_WIDTH", 1280.0),
            "windowHeight": qa_dimension("VELA_NATIVE_QA_HEIGHT", 840.0),
        }))
        .expect("fixture credentials sent to own window");
        let mut captures = Vec::new();
        let result = loop {
            match tokio::time::timeout(std::time::Duration::from_secs(90), eval.recv::<Value>())
                .await
            {
                Ok(Ok(message)) if message["resize"].is_object() => {
                    let width = message["resize"]["width"]
                        .as_u64()
                        .filter(|width| (320..=4096).contains(width));
                    let height = message["resize"]["height"]
                        .as_u64()
                        .filter(|height| (320..=4096).contains(height));
                    let (Some(width), Some(height)) = (width, height) else {
                        let received = message["resize"].to_string();
                        let bounded: String = received.chars().take(200).collect();
                        break json!({
                            "ok":false,
                            "failure":format!("Invalid resize request: {bounded}")
                        });
                    };
                    let _ = tokio::task::spawn_blocking(move || {
                        // Tiling compositors may ignore application size requests.
                        // Float/resize only this fixture's exact owned surface.
                        resize_owned_window(width, height);
                    })
                    .await;
                    let _ = eval.send(json!({"continue":true}));
                }
                Ok(Ok(message)) if message["capture"].is_string() => {
                    let name = message["capture"].as_str().unwrap().to_owned();
                    if !known_capture(&name) {
                        break json!({"ok":false,"failure":"Unknown capture stage"});
                    }
                    let output = report.with_file_name(format!("native-{name}.png"));
                    let pid = std::process::id().to_string();
                    let viewport = &message["layout"]["viewport"];
                    let scale = viewport["devicePixelRatio"].as_f64().unwrap_or(1.0);
                    let expected = (
                        (viewport["width"].as_f64().unwrap_or(0.0) * scale).round() as u32,
                        (viewport["height"].as_f64().unwrap_or(0.0) * scale).round() as u32,
                    );
                    let capture = tokio::task::spawn_blocking(move || {
                        let status = if std::env::var("VELA_NATIVE_QA_BACKEND").as_deref()
                            == Ok("wayland")
                        {
                            let clients = std::process::Command::new("hyprctl")
                                .args(["clients", "-j"])
                                .output()
                                .map_err(|_| "Hyprland query failed")?;
                            let clients: Vec<Value> = serde_json::from_slice(&clients.stdout)
                                .map_err(|_| "Invalid Hyprland clients")?;
                            let owned = clients
                                .iter()
                                .find(|c| c["pid"].as_u64() == Some(std::process::id() as u64))
                                .ok_or("Owned Wayland surface missing")?;
                            if owned["xwayland"].as_bool() != Some(false) {
                                return Err("Fixture is not native Wayland");
                            }
                            let x = owned["at"][0].as_i64().ok_or("Invalid window x")?;
                            let y = owned["at"][1].as_i64().ok_or("Invalid window y")?;
                            let width = owned["size"][0]
                                .as_u64()
                                .filter(|v| *v > 0 && *v <= 8192)
                                .ok_or("Invalid window width")?;
                            let height = owned["size"][1]
                                .as_u64()
                                .filter(|v| *v > 0 && *v <= 8192)
                                .ok_or("Invalid window height")?;
                            std::process::Command::new("grim")
                                .args([
                                    "-g",
                                    &format!("{x},{y} {width}x{height}"),
                                    "-s",
                                    &scale.to_string(),
                                ])
                                .arg(&output)
                                .status()
                                .map_err(|_| "Wayland capture failed")?
                        } else {
                            let windows = std::process::Command::new("xdotool")
                                .args(["search", "--onlyvisible", "--pid", &pid])
                                .output()
                                .map_err(|_| "Window lookup failed")?;
                            let ids = String::from_utf8_lossy(&windows.stdout);
                            let id = ids.lines().next().ok_or("Owned window was not found")?;
                            std::process::Command::new("import")
                                .args(["-window", id])
                                .arg(&output)
                                .status()
                                .map_err(|_| "Screenshot tool unavailable")?
                        };
                        if !status.success() {
                            return Err("Screenshot failed");
                        }
                        // X11 capture tools can silently crop a partly off-screen
                        // window. A cropped image is not full-viewport evidence.
                        let png = std::fs::read(&output).map_err(|_| "Screenshot unreadable")?;
                        if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
                            return Err("Screenshot is not PNG");
                        }
                        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
                        let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
                        if (width, height) != expected {
                            return Err("Screenshot was cropped or has an unexpected scale");
                        }
                        Ok::<_, &str>(output)
                    })
                    .await;
                    let error = match &capture {
                        Ok(Err(message)) => Some(*message),
                        Err(_) => Some("Screenshot worker failed"),
                        _ => None,
                    };
                    captures.push(json!({"name":name,"passed":matches!(capture,Ok(Ok(_))),"error":error,"layout":message["layout"]}));
                    let _ = eval.send(json!({"continue":true}));
                }
                Ok(Ok(message)) if message["engineStreams"].as_bool() == Some(true) => {
                    // TR10: count this window process's native output streams on the
                    // QA-owned Pulse/PipeWire sink. The user's default sink is never
                    // inspected or changed; the report carries bounded text only.
                    let evidence = tokio::task::spawn_blocking(audio_stream_report)
                        .await
                        .unwrap_or_else(|_| json!({"error":"audio stream query failed"}));
                    let _ = eval.send(json!({"continue":true,"engineStreams":evidence}));
                }
                Ok(Ok(message)) if message["handshake"].is_string() => {
                    let kind = message["handshake"].as_str().unwrap_or_default();
                    let (request, ack) = match kind {
                        "server-stop" => (
                            std::env::var("VELA_NATIVE_QA_STOP_REQUEST").ok(),
                            std::env::var("VELA_NATIVE_QA_STOPPED_ACK").ok(),
                        ),
                        "server-start" => (
                            std::env::var("VELA_NATIVE_QA_START_REQUEST").ok(),
                            std::env::var("VELA_NATIVE_QA_STARTED_ACK").ok(),
                        ),
                        _ => (None, None),
                    };
                    let (Some(request), Some(ack)) = (request, ack) else {
                        break json!({"ok":false,"failure":"Unknown native fixture handshake"});
                    };
                    if std::fs::write(&request, kind).is_err() {
                        break json!({"ok":false,"failure":"Could not request fixture server transition"});
                    }
                    let waited = tokio::task::spawn_blocking(move || {
                        for _ in 0..200 {
                            if std::path::Path::new(&ack).exists() {
                                return true;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        false
                    })
                    .await
                    .unwrap_or(false);
                    if !waited {
                        break json!({"ok":false,"failure":"Fixture server transition timed out"});
                    }
                    let _ = eval.send(json!({"continue":true}));
                }
                Ok(Ok(message)) => break message,
                _ => {
                    break json!({"ok":false,"failure":"Native UI driver timed out or evaluation failed"});
                }
            }
        };
        let output = json!({"result":result,"screenshots":captures,"engine":"Rodio/CPAL, owned isolated null sink","driver":"Dioxus document evaluation in actual WebKitGTK window"});
        std::fs::write(report, serde_json::to_vec_pretty(&output).unwrap())
            .expect("write fixture report");
        dioxus::desktop::window().close();
    });
    rsx! { vela_desktop::ui::App {} }
}

/// Fixed capture stages plus the per-viewport artwork evidence. The width suffix is
/// bounded so a driver message can never choose an arbitrary output path.
fn known_capture(name: &str) -> bool {
    matches!(
        name,
        "library-dark"
            | "typography-dark"
            | "library-wide-dark"
            | "library-ultrawide-dark"
            | "queue-light"
            | "compact-light"
            | "favorites-dark"
            | "playlists-dark"
            | "albums-dark"
            | "public-dark"
            | "public-search-dark"
            | "player-band-dark"
            | "player-band-narrow-light"
    ) || ["library-artwork-", "player-artwork-"]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix)
                .and_then(|width| width.parse::<u32>().ok())
                .is_some_and(|width| (320..=4096).contains(&width))
        })
}

/// Count the native output streams this QA window process owns, observed from the
/// QA-owned Pulse/PipeWire sink named by `PULSE_SINK`. One streaming client is the
/// observable evidence for the one `AudioEngine` output authority; the probe never
/// inspects, selects or changes the user's default output and reports bounded text.
fn audio_stream_report() -> Value {
    #[derive(Default)]
    struct Stream {
        sink: Option<String>,
        process: Option<String>,
        name: Option<String>,
    }
    fn finish(current: &mut Stream) -> Option<Stream> {
        let stream = std::mem::take(current);
        stream.sink.is_some().then_some(stream)
    }
    let pid = std::process::id();
    let sink_name = std::env::var("PULSE_SINK").unwrap_or_default();
    let sink_index = std::process::Command::new("pactl")
        .args(["list", "short", "sinks"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| {
            text.lines().find_map(|line| {
                let mut columns = line.split('\t');
                let index = columns.next()?.trim().to_owned();
                let name = columns.next()?.trim();
                (!index.is_empty() && name == sink_name).then_some(index)
            })
        });
    let output = match std::process::Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => {
            return json!({
                "sink": sink_name,
                "sinkIndex": sink_index,
                "error": "pactl unavailable",
            });
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut streams: Vec<Stream> = Vec::new();
    let mut current = Stream::default();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sink Input #") {
            if let Some(stream) = finish(&mut current) {
                streams.push(stream);
            }
            current = Stream::default();
        } else if let Some(value) = trimmed.strip_prefix("Sink: ") {
            current.sink = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("application.process.id = ") {
            current.process = Some(value.trim().trim_matches('"').to_owned());
        } else if let Some(value) = trimmed.strip_prefix("application.name = ") {
            current.name = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    if let Some(stream) = finish(&mut current) {
        streams.push(stream);
    }
    let pid_text = pid.to_string();
    let ours: Vec<&Stream> = streams
        .iter()
        .filter(|stream| match sink_index.as_deref() {
            Some(index) => stream.sink.as_deref() == Some(index),
            None => stream.process.as_deref() == Some(pid_text.as_str()),
        })
        .collect();
    let process_exposed = ours.iter().any(|stream| stream.process.is_some());
    let process_streams = ours
        .iter()
        .filter(|stream| stream.process.as_deref() == Some(pid_text.as_str()))
        .count();
    let names: Vec<Value> = ours
        .iter()
        .take(4)
        .map(|stream| {
            json!({
                "sink": stream.sink.clone(),
                "process": stream.process.clone(),
                "name": stream.name.clone(),
            })
        })
        .collect();
    json!({
        "sink": sink_name,
        "sinkIndex": sink_index,
        "sinkInputs": ours.len(),
        "totalSinkInputs": streams.len(),
        "processStreams": if process_exposed { Some(process_streams) } else { None },
        "pid": pid,
        "streams": names,
    })
}

/// Optional requested fixture viewport. Invalid or unbounded values keep the documented
/// defaults instead of being passed to the window/compositor.
fn qa_dimension(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && (320.0..=4096.0).contains(value))
        .unwrap_or(default)
}

fn resize_owned_window(width: u64, height: u64) {
    let pid = std::process::id().to_string();
    if let Ok(clients) = std::process::Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        && let Ok(clients) = serde_json::from_slice::<Vec<Value>>(&clients.stdout)
        && let Some(owned) = clients
            .iter()
            .find(|client| client["pid"].as_u64() == Some(std::process::id() as u64))
        && let Some(address) = owned["address"].as_str()
        && address.starts_with("0x")
        && address[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        // Hyprland 0.56 evaluates dispatched actions as Lua. Float only this
        // fixture's exact address; no rule or compositor state is persisted.
        // A tiled surface ignores X11 resize requests.
        // Hyprland 0.55+ exposes dispatchers as Lua closures; `action="set"`
        // is not a float action, so the surface stayed tiled and ignored the
        // X11 resize. `enable` is the documented action.
        let code = format!(
            "hl.dispatch(hl.dsp.window.float({{action='enable', window='address:{address}'}}))"
        );
        let _ = std::process::Command::new("hyprctl")
            .args(["eval", &code])
            .output();
    }
    if let Ok(windows) = std::process::Command::new("xdotool")
        .args(["search", "--onlyvisible", "--pid", &pid])
        .output()
        && let Some(id) = String::from_utf8_lossy(&windows.stdout).lines().next()
    {
        let _ = std::process::Command::new("xdotool")
            .args(["windowsize", id, &width.to_string(), &height.to_string()])
            .output();
        let _ = std::process::Command::new("xdotool")
            .args(["windowmove", id, "0", "0"])
            .output();
    }
    std::thread::sleep(std::time::Duration::from_millis(700));
}

fn main() {
    let sink = std::env::var("VELA_NATIVE_TEST_SINK").expect("owned test sink name");
    assert_eq!(sink, "vela_native_test_null");
    assert_eq!(
        std::env::var("PULSE_SINK").as_deref(),
        Ok(sink.as_str()),
        "QA must use the script-owned null sink, never the user's default output"
    );
    let alsa = std::env::var("ALSA_CONFIG_PATH").expect("isolated ALSA configuration");
    assert!(
        std::fs::read_to_string(alsa).is_ok_and(|config| config.contains(&sink)),
        "isolated ALSA configuration must route to the owned null sink"
    );
    let window = vela_desktop::window::builder("Vela Music — isolated native QA").with_inner_size(
        dioxus::desktop::LogicalSize::new(
            qa_dimension("VELA_NATIVE_QA_WIDTH", 1280.0),
            qa_dimension("VELA_NATIVE_QA_HEIGHT", 840.0),
        ),
    );
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(None)
                .with_disable_dma_buf_on_wayland(false)
                .with_navigation_handler(|_| false),
        )
        .launch(Harness);
}
