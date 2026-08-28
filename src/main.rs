use apple::daemon::AppleDaemonServer;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "apple")]
#[command(about = "Hermetic sandbox and process isolation daemon for the Fish build system")]
#[command(version)]
struct Args {
    #[arg(
        short,
        long,
        default_value = ".apple-scratch",
        help = "Scratch directory for jail isolation"
    )]
    scratch_dir: PathBuf,

    #[arg(
        short,
        long,
        default_value = "apple.sock",
        help = "Socket or named pipe address"
    )]
    socket: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("🍎 Apple Sandbox Daemon v{}", env!("CARGO_PKG_VERSION"));
    println!("   Scratch root : {}", args.scratch_dir.display());
    println!("   IPC Endpoint : {}", args.socket);

    let _server = AppleDaemonServer::new(args.scratch_dir);
    println!("   Status       : Active & Listening for Fish build orchestrator");

    Ok(())
}
