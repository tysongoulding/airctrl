use airctrl_lan::{get_lan_ip, start_lan_server, LanServerConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let lan_ip = get_lan_ip()?;

    // 1. Resolve port: CLI flag --port/-p takes precedence, then AIRCTRL_PORT, default 8080
    let mut port: u16 = std::env::var("AIRCTRL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // 2. Resolve artifact path: CLI flag --artifact/-a takes precedence, then AIRCTRL_ARTIFACT_PATH
    let mut artifact_path = std::env::var("AIRCTRL_ARTIFACT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("dist/airctrl.exe"));

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" if i + 1 < args.len() => {
                if let Ok(p) = args[i + 1].parse() {
                    port = p;
                }
                i += 1;
            }
            "--artifact" | "-a" if i + 1 < args.len() => {
                artifact_path = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let config = LanServerConfig {
        bind_ip: std::net::Ipv4Addr::new(0, 0, 0, 0),
        port,
        advertised_ip: Some(lan_ip),
        artifact_path,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let handle = start_lan_server(config).await?;
    println!("============================================================");
    println!(" airctrl LAN Distribution Daemon Active");
    println!(" Listening on:       http://0.0.0.0:{}", handle.local_addr.port());
    println!(" Advertised Base:    {}", handle.base_url);
    println!(" Windows Run One-Liner:");
    println!("   irm {}/run.ps1 | iex", handle.base_url);
    println!("============================================================");

    tokio::signal::ctrl_c().await?;
    println!("\nShutting down airctrl LAN daemon...");
    handle.shutdown().await;
    Ok(())
}
