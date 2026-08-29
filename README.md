# 🍎 Apple: Hermetic Sandbox & Process Isolation Daemon for Fish

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](README.md) | [Tiếng Việt](docs/vi/README.md) | [日本語](docs/ja/README.md) | [简体中文](docs/zh-hans/README.md) | [繁體中文](docs/zh-hant/README.md)

---

## 🎯 Overview

**Apple** is a process-level hermetic sandbox and isolation daemon that
complements the [Fish](https://github.com/requla11/fish) build orchestration
engine. While Fish coordinates dependency graphs, caching, and parallel
scheduling, Apple wraps individual build commands in a controlled environment:
a scrubbed variable set, a scratch working copy, toolchain-level offline
flags, and an enforced timeout (with a Windows Job Object on Windows).

Apple ships as both a Rust library (consumed by `fish-sandbox`) and a
standalone CLI/daemon.

> **Note on the name:** "Apple" is a companion project name for Fish 🐟.
> This project is an independent open-source tool and is **not affiliated
> with, endorsed, or sponsored by Apple Inc.**

## ⚡ What Apple actually does

1. **Hard-link mirror sandbox (`apple::isolation::fs`)**:
   * Mirrors source trees into a per-task jail directory using hard links
     (with an automatic copy fallback across filesystems).
   * Compiler writes land in the jail, leaving the original tree untouched.

2. **Environment scrubbing (`apple::isolation::env`)**:
   * Strips all environment variables except an allow-list (plus `FISH_*` and
     `APPLE_*` prefixes) and points `TMPDIR`/`TEMP`/`TMP` at the jail.

3. **Best-effort network discouragement (`apple::isolation::net`)**:
   * Injects blackhole proxy variables and offline flags honored by Cargo,
     Go, pip, and npm (`CARGO_NET_OFFLINE`, `GOPROXY=off`, ...).
   * **This is not a firewall.** A process that ignores proxy variables still
     has network access. Kernel-level enforcement (network namespaces) is not
     implemented.

4. **Process isolation (`apple::isolation::process`)**:
   * **Windows**: Job Object with `KILL_ON_JOB_CLOSE` and an optional memory
     ceiling; `CREATE_NO_WINDOW` for child processes.
   * **Unix**: `setpgid` process-group isolation and a hard timeout.
   * This is user-space process isolation — no namespaces, seccomp, or
     AppContainer.

5. **Dual-pass determinism check (`apple::verifier`)**:
   * Runs the same build twice in fresh jails; the second pass runs with
     perturbed locale/time variables (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`).
   * Compares BLAKE3 hashes of the artifact. This is a self-declared
     reproducibility check, **not** a SLSA attestation.

6. **Audit records (`apple::audit`)**:
   * The daemon persists execution results (exit code, duration, violations)
     as JSON under `<scratch>/audit/<task_id>.json` for inspection by the CLI.

7. **Violation checking (`apple::monitor`)**:
   * A path-prefix policy checker available as a library. It is not wired to
     live syscall/process I/O interception.

## 🚀 CLI Reference

### 1. Start the IPC daemon
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```
Serves newline-delimited JSON (`DaemonMessage`) over a Unix socket or a
Windows named pipe until a `Shutdown` message or Ctrl+C.

### 2. One-shot sandboxed execution
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Verify deterministic output
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```
Requires the build to produce the artifact **inside** the jail so both
passes can be hashed.

### 4. Check daemon status
```bash
apple status --socket apple.sock
```
Pings the real daemon over IPC and reports reachability, version, and the
active sandbox count.

### 5. Inspect audit records
```bash
apple audit <task_id>
apple telemetry <task_id>
```
Reads JSON records previously written by the daemon. If no record exists,
the CLI reports that — it never prints placeholder numbers.

### 6. Auto-detect a language profile
```bash
apple profile-detect --dir .
```

## 🧪 Known Limitations

* No kernel-level sandboxing (no namespaces/seccomp on Linux, no AppContainer
  or AppLocker on Windows).
* Network lockdown is advisory (env-var based), not enforced.
* The violation monitor is a library-only path checker, not a runtime I/O
  interceptor.
* Peak memory and CPU-time sampling are not implemented; telemetry reports
  what the runner actually knows (exit code, duration).
* The determinism verifier requires the artifact to be produced inside the
  jail; it cannot hash artifacts written outside the sandbox.
* IPC is single-host only (Unix socket / named pipe).

## 📄 License & Disclaimer

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

> **Disclaimer:** This project is an independent open-source tool and is not
> affiliated with, endorsed, or sponsored by Apple Inc.
