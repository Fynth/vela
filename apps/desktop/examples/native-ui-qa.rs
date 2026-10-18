//! Opt-in real-window integration driver. Never compiled into the shipping app.
use dioxus::{
    desktop::{Config, LogicalSize, WindowBuilder},
    prelude::*,
};
use serde_json::{Value, json};
use std::path::PathBuf;

#[allow(non_snake_case)]
fn Harness() -> Element {
    use_future(|| async {
        let report =
            PathBuf::from(std::env::var_os("VELA_NATIVE_QA_REPORT").expect("fixture report path"));
        let mut eval = document::eval(include_str!("native-ui-qa.js"));
        eval.send(json!({
            "origin": std::env::var("VELA_DESKTOP_SERVER_URL").expect("isolated fixture origin"),
            "username": std::env::var("VELA_NATIVE_QA_USERNAME").expect("fixture username"),
            "password": std::env::var("VELA_NATIVE_QA_PASSWORD").expect("fixture password"),
            "closeWhilePlaying": std::env::var("VELA_NATIVE_QA_CLOSE_PLAYING").as_deref() == Ok("1"),
            "peerOrigin": std::env::var("VELA_NATIVE_QA_PEER_ORIGIN").unwrap_or_default(),
            "peer2Origin": std::env::var("VELA_NATIVE_QA_PEER2_ORIGIN").unwrap_or_default(),
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
                    if !matches!(
                        name.as_str(),
                        "library-dark"
                            | "queue-light"
                            | "compact-light"
                            | "favorites-dark"
                            | "playlists-dark"
                            | "albums-dark"
                            | "public-dark"
                            | "public-search-dark"
                    ) {
                        break json!({"ok":false,"failure":"Unknown capture stage"});
                    }
                    let output = report.with_file_name(format!("native-{name}.png"));
                    let pid = std::process::id().to_string();
                    let capture = tokio::task::spawn_blocking(move || {
                        let windows = std::process::Command::new("xdotool")
                            .args(["search", "--onlyvisible", "--pid", &pid])
                            .output()
                            .map_err(|_| "Window lookup failed")?;
                        let ids = String::from_utf8_lossy(&windows.stdout);
                        let id = ids.lines().next().ok_or("Owned window was not found")?;
                        let status = std::process::Command::new("import")
                            .args(["-window", id])
                            .arg(&output)
                            .status()
                            .map_err(|_| "Screenshot tool unavailable")?;
                        if !status.success() {
                            return Err("Screenshot failed");
                        }
                        Ok::<_, &str>(output)
                    })
                    .await;
                    captures.push(json!({"name":name,"passed":matches!(capture,Ok(Ok(_)))}));
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
    let window = WindowBuilder::new()
        .with_title("Vela Music — isolated native QA")
        .with_inner_size(LogicalSize::new(1280., 840.));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_navigation_handler(|_| false),
        )
        .launch(Harness);
}
