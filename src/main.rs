use apple::audit::audit_record_path;
use apple::daemon::AppleDaemonServer;
use apple::profile_detector::ProfileDetector;
use apple::protocol::{ExecutionRequest, ExecutionResult, IsolationLevel, SandboxProfile};
use apple::verifier::DeterminismVerifier;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DEFAULT_SCRATCH: &str = ".apple-scratch";
const DEFAULT_SOCKET: &str = "apple.sock";

#[derive(Parser, Debug)]
#[command(name = "apple")]
#[command(about = "Hermetic sandbox and process isolation daemon for the Fish build system")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Daemon {
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
    },
    Run {
        #[arg(short, long, help = "Enforce zero-trust offline network lockdown")]
        offline: bool,

        #[arg(short, long, help = "Memory limit in megabytes")]
        memory_limit_mb: Option<u64>,

        #[arg(short, long, help = "Timeout in seconds")]
        timeout_seconds: Option<u64>,

        #[arg(short, long, help = "Working directory")]
        workdir: Option<PathBuf>,

        #[arg(
            last = true,
            required = true,
            help = "Command and arguments to execute in sandbox"
        )]
        command: Vec<String>,
    },
    Status {
        #[arg(
            short,
            long,
            default_value = "apple.sock",
            help = "Socket or named pipe address"
        )]
        socket: String,
    },
    Audit {
        #[arg(help = "Task ID to inspect audit logs for")]
        task_id: String,
    },
    Telemetry {
        #[arg(help = "Task ID to inspect resource telemetry for")]
        task_id: String,
    },
    ProfileDetect {
        #[arg(short, long, help = "Project root directory to analyze")]
        dir: Option<PathBuf>,
    },
    VerifyReproducible {
        #[arg(short, long, help = "Path to the generated artifact to verify")]
        artifact: PathBuf,

        #[arg(short, long, help = "Enforce zero-trust offline network lockdown")]
        offline: bool,

        #[arg(short, long, help = "Working directory")]
        workdir: Option<PathBuf>,

        #[arg(
            last = true,
            required = true,
            help = "Command and arguments to execute for reproducible verification"
        )]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Status {
        socket: DEFAULT_SOCKET.to_string(),
    }) {
        Commands::Daemon {
            scratch_dir,
            socket,
        } => {
            println!("🍎 Apple Sandbox Daemon v{}", env!("CARGO_PKG_VERSION"));
            println!("   Scratch root : {}", scratch_dir.display());
            println!("   IPC Endpoint : {}", socket);

            let server = std::sync::Arc::new(AppleDaemonServer::new(scratch_dir));
            println!("   Status       : Listening (Ctrl+C to stop)");
            server.serve(&socket).await?;
        }
        Commands::Run {
            offline,
            memory_limit_mb,
            timeout_seconds,
            workdir,
            command,
        } => {
            let cwd = workdir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let server = AppleDaemonServer::new(PathBuf::from(DEFAULT_SCRATCH));

            let mut profile = SandboxProfile::default();
            if offline {
                profile.allow_network = false;
            }
            if let Some(mem) = memory_limit_mb {
                profile.memory_limit_mb = Some(mem);
            }
            if let Some(to) = timeout_seconds {
                profile.timeout_seconds = Some(to);
            }
            profile.level = IsolationLevel::FullHermetic;

            let request = ExecutionRequest {
                task_id: format!("cli_exec_{}", std::process::id()),
                working_dir: cwd,
                argv: command,
                env: std::env::vars().collect::<HashMap<_, _>>(),
                profile,
            };

            let res = server.execute_task(request).await;
            if !res.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&res.stdout));
            }
            if !res.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&res.stderr));
            }
            std::process::exit(res.exit_code);
        }
        Commands::Status { socket } => {
            println!("🍎 Apple Sandbox Daemon Status");
            println!("   IPC Endpoint : {}", socket);
            match AppleDaemonServer::ping_endpoint(&socket).await {
                Ok((version, active)) => {
                    println!("   Reachable    : yes");
                    println!("   Version      : v{version}");
                    println!("   Active Jobs  : {active}");
                }
                Err(_) => {
                    println!("   Reachable    : no (is the daemon running on this endpoint?)");
                }
            }
        }
        Commands::Audit { task_id } => {
            println!("🍎 Apple Audit Record");
            println!("   Task ID : {task_id}");
            match load_audit_record(&task_id) {
                Some(res) => {
                    println!("   Exit Code          : {}", res.exit_code);
                    println!("   Duration           : {} ms", res.execution_duration_ms);
                    println!("   Violations         : {}", res.violations.len());
                    println!("   Hermetic Guarantee : {}", res.hermetic_guarantee);
                    for v in &res.violations {
                        println!("   - [{}] {}", v.operation, v.description);
                    }
                }
                None => println!(
                    "   Status  : No audit record found (the daemon writes records under {DEFAULT_SCRATCH}/audit/)"
                ),
            }
        }
        Commands::Telemetry { task_id } => {
            println!("🍎 Apple Resource Telemetry");
            println!("   Task ID    : {task_id}");
            match load_audit_record(&task_id) {
                Some(res) => {
                    println!("   Exit Code  : {}", res.exit_code);
                    println!("   Duration   : {} ms", res.execution_duration_ms);
                    println!("   Peak RAM   : {} bytes", res.peak_memory_bytes);
                    println!(
                        "   Note       : CPU time and peak-RAM sampling are not implemented; the runner reports 0."
                    );
                }
                None => println!("   Status     : No audit record found for this task"),
            }
        }
        Commands::ProfileDetect { dir } => {
            let root = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let lang = ProfileDetector::detect_language(&root);
            let profile = ProfileDetector::auto_generate_profile(&root);

            println!("🍎 Apple Language Profile Auto-Detection");
            println!("   Root Directory : {}", root.display());
            println!("   Detected Type  : {:?}", lang);
            println!("   Synthesized    : {}", profile.name);
            println!(
                "   Network Access : {}",
                if profile.allow_network {
                    "Allowed"
                } else {
                    "Blocked (best-effort toolchain flags)"
                }
            );
            println!("   Whitelisted Env: {:?}", profile.whitelisted_env);
        }
        Commands::VerifyReproducible {
            artifact,
            offline,
            workdir,
            command,
        } => {
            let cwd = workdir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let verifier = DeterminismVerifier::new(PathBuf::from(DEFAULT_SCRATCH));

            let mut profile = SandboxProfile::default();
            if offline {
                profile.allow_network = false;
            }
            profile.level = IsolationLevel::FullHermetic;

            let request = ExecutionRequest {
                task_id: format!("verify_{}", std::process::id()),
                working_dir: cwd,
                argv: command,
                env: std::env::vars().collect::<HashMap<_, _>>(),
                profile,
            };

            let report = verifier.verify_reproducible(request, &artifact).await?;

            println!("🍎 Apple Deterministic Build Verification");
            println!("   Artifact  : {}", report.artifact_path);
            println!("   Pass 1 Hdr: {}", report.pass1_hash);
            println!("   Pass 2 Hdr: {}", report.pass2_hash);

            if report.is_deterministic {
                println!("   Verdict   : Deterministic (bit-for-bit identical output) ✅");
            } else {
                eprintln!("   Verdict   : Non-deterministic output detected ❌");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Load an audit record previously persisted by the daemon from the default
/// scratch directory. Returns `None` when no record exists — the CLI never
/// invents placeholder data.
fn load_audit_record(task_id: &str) -> Option<ExecutionResult> {
    let path = audit_record_path(Path::new(DEFAULT_SCRATCH), task_id);
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}
