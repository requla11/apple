use apple::attestation::{AttestationEnvelope, AttestationSigner};
use apple::audit::audit_record_path;
use apple::daemon::AppleDaemonServer;
use apple::profile_detector::ProfileDetector;
use apple::protocol::{
    ExecutionRequest, ExecutionResult, IsolationLevel, MountKind, MountRule, SandboxProfile,
};
use apple::provenance::SlsaProvenanceGenerator;
use apple::sbom::SbomGenerator;
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
    Provenance {
        #[arg(short, long, help = "Task ID from previous run audit")]
        task_id: String,

        #[arg(short, long, help = "Path to output artifact")]
        artifacts: Vec<PathBuf>,

        #[arg(short, long, help = "Output path to write provenance JSON")]
        output: Option<PathBuf>,
    },
    Sbom {
        #[arg(
            short,
            long,
            default_value = "spdx",
            help = "Format: spdx or cyclonedx"
        )]
        format: String,

        #[arg(short, long, help = "Task ID")]
        task_id: String,

        #[arg(short, long, help = "Path to artifacts")]
        artifacts: Vec<PathBuf>,

        #[arg(short, long, help = "Output path to write SBOM JSON")]
        output: Option<PathBuf>,
    },
    Attest {
        #[arg(short, long, help = "Path to provenance file")]
        provenance: PathBuf,

        #[arg(short, long, help = "Secret key hex string (32 bytes)")]
        secret_key: String,

        #[arg(
            short,
            long,
            default_value = "apple-builder-key-1",
            help = "Key ID identifier"
        )]
        key_id: String,

        #[arg(short, long, help = "Verify instead of sign")]
        verify: bool,

        #[arg(short, long, help = "Path to envelope JSON if verifying")]
        envelope: Option<PathBuf>,
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
            let scratch = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(DEFAULT_SCRATCH);
            let server = AppleDaemonServer::new(scratch);

            let mut profile = SandboxProfile {
                mount_rules: vec![MountRule {
                    source: cwd.clone(),
                    target: PathBuf::from("."),
                    kind: MountKind::ReadOnly,
                }],
                ..SandboxProfile::default()
            };
            profile.allow_network = !offline;
            if let Some(mem) = memory_limit_mb {
                profile.memory_limit_mb = Some(mem);
            }
            if let Some(to) = timeout_seconds {
                profile.timeout_seconds = Some(to);
            }
            profile.level = IsolationLevel::FullHermetic;

            let task_id = format!("cli_exec_{}", std::process::id());
            let request = ExecutionRequest {
                task_id: task_id.clone(),
                working_dir: cwd,
                argv: command,
                env: std::env::vars().collect::<HashMap<_, _>>(),
                profile,
                keep_jail: true,
            };

            let res = server.execute_task(request).await;
            if !res.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&res.stdout));
            }
            if !res.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&res.stderr));
            }
            println!("🍎 sandbox jail kept at {DEFAULT_SCRATCH}/jail_{task_id}");
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
                keep_jail: false,
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
        Commands::Provenance {
            task_id,
            artifacts,
            output,
        } => {
            let Some(res) = load_audit_record(&task_id) else {
                eprintln!("Error: no audit record found for task {task_id}");
                std::process::exit(1);
            };

            let req = ExecutionRequest {
                task_id: task_id.clone(),
                working_dir: std::env::current_dir().unwrap_or_default(),
                argv: Vec::new(),
                env: HashMap::new(),
                profile: SandboxProfile::default(),
                keep_jail: false,
            };

            let statement = SlsaProvenanceGenerator::generate(&req, &res, &artifacts)?;
            let json = serde_json::to_string_pretty(&statement)?;

            if let Some(out_path) = output {
                std::fs::write(&out_path, &json)?;
                println!("🍎 SLSA v1.0 Provenance written to {}", out_path.display());
            } else {
                println!("{json}");
            }
        }
        Commands::Sbom {
            format,
            task_id,
            artifacts,
            output,
        } => {
            let json = match format.to_lowercase().as_str() {
                "spdx" => {
                    let doc = SbomGenerator::generate_spdx(&task_id, &artifacts)?;
                    serde_json::to_string_pretty(&doc)?
                }
                "cyclonedx" => {
                    let bom = SbomGenerator::generate_cyclonedx(&task_id, &artifacts)?;
                    serde_json::to_string_pretty(&bom)?
                }
                other => {
                    eprintln!("Unsupported SBOM format: {other}. Use 'spdx' or 'cyclonedx'");
                    std::process::exit(1);
                }
            };

            if let Some(out_path) = output {
                std::fs::write(&out_path, &json)?;
                println!("🍎 SBOM ({format}) written to {}", out_path.display());
            } else {
                println!("{json}");
            }
        }
        Commands::Attest {
            provenance,
            secret_key,
            key_id,
            verify,
            envelope,
        } => {
            let key_bytes = decode_hex_key(&secret_key)?;

            if verify {
                let Some(env_path) = envelope else {
                    eprintln!("Error: --envelope path is required for verification");
                    std::process::exit(1);
                };
                let content = std::fs::read_to_string(&env_path)?;
                let env: AttestationEnvelope = serde_json::from_str(&content)?;
                let valid = AttestationSigner::verify_envelope(&env, &key_bytes)?;
                if valid {
                    println!("🍎 Attestation signature verified successfully ✅");
                } else {
                    eprintln!("❌ Attestation signature verification FAILED");
                    std::process::exit(1);
                }
            } else {
                let content = std::fs::read_to_string(&provenance)?;
                let stmt = serde_json::from_str(&content)?;
                let env = AttestationSigner::sign_statement(&stmt, &key_bytes, &key_id)?;
                let json = serde_json::to_string_pretty(&env)?;
                println!("{json}");
            }
        }
    }

    Ok(())
}

fn load_audit_record(task_id: &str) -> Option<ExecutionResult> {
    let path = audit_record_path(Path::new(DEFAULT_SCRATCH), task_id);
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

fn decode_hex_key(hex_str: &str) -> anyhow::Result<[u8; 32]> {
    let raw = hex_str.trim().as_bytes();
    let mut key = [0u8; 32];
    if raw.len() != 64 {
        anyhow::bail!("secret key must be exactly 64 hex characters (32 bytes)");
    }
    for i in 0..32 {
        let h1 = parse_hex_digit(raw[i * 2])?;
        let h2 = parse_hex_digit(raw[i * 2 + 1])?;
        key[i] = (h1 << 4) | h2;
    }
    Ok(key)
}

fn parse_hex_digit(byte: u8) -> anyhow::Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => anyhow::bail!("invalid hex character"),
    }
}
