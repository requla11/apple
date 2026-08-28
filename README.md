# 🍎 Apple: Kernel-Level Hermetic Sandbox & Process Isolation Daemon

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](README.md) | [Tiếng Việt](docs/vi/README.md) | [日本語](docs/ja/README.md) | [简体中文](docs/zh-hans/README.md) | [繁體中文](docs/zh-hant/README.md)

---

## 🎯 Overview

**Apple** is a dedicated kernel-level hermetic sandbox and zero-trust process isolation daemon engineered to complement the [Fish](https://github.com/requla11/fish) polyglot build orchestration engine.

While Fish coordinates high-throughput dependency DAGs, incremental caching, and parallel task scheduling, **Apple** acts as the elevated system hypervisor, ensuring that every compiler invocation executes inside an airtight, leak-free, reproducible environment.

```text
┌─────────────────────────────────────────────────────────────┐
│                 🐟 Fish Build Orchestrator                  │
└──────────────────────────────┬──────────────────────────────┘
                               │ IPC (Unix Domain Socket / Named Pipe)
┌──────────────────────────────▼──────────────────────────────┐
│                   🍎 Apple Sandbox Daemon                   │
├──────────────────────────────┬──────────────────────────────┤
│  Hermetic Filesystem Manager │  Network Lockdown Controller │
│  (Readonly Jails & Overlay)  │  (Zero-Trust Offline Mirror) │
├──────────────────────────────┼──────────────────────────────┤
│  Process Isolation Runner    │  Real-Time Violation Monitor │
│  (Job Objects & Namespaces)  │  (IO Auditing & Telemetry)   │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Core Capabilities

1. **Hermetic Filesystem Jails (`apple::isolation::fs`)**:
   * Mounts source trees in isolated read-only views.
   * Redirects temporary compiler writes into disposable scratch directories, preventing any residue in user workspaces.

2. **Zero-Trust Network Lockdown (`apple::isolation::net`)**:
   * Blackholes unauthorized outbound network requests during compilation tasks to guarantee 100% reproducible build artifacts.

3. **OS-Native Kernel Process Isolation (`apple::isolation::process`)**:
   * **Windows**: Implements Native Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, strict RAM ceilings).
   * **Unix / Linux**: Enforces process group leader isolation (`setpgid`) and timeout enforcement.

4. **Hermetic Environment Sanitization (`apple::isolation::env`)**:
   * Scrubs non-deterministic environment variables (`USER`, `PWD`, `HOME`, `TEMP`, `LOGNAME`) while preserving compiler flags (`RUSTFLAGS`, `CFLAGS`, `NODE_ENV`).

5. **Real-Time Violation Monitor & Audit Store (`apple::monitor`, `apple::audit`)**:
   * Inspects process I/O and immediately flags any compiler attempt to read un-declared headers or leaked build secrets.

---

## 🚀 CLI Reference

### 1. One-Shot Sandboxed Execution
```bash
# Run any command inside a hermetic offline jail with memory and timeout limits
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 2. Background Daemon Mode
```bash
# Start the IPC daemon for Fish build orchestration
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 3. Check Daemon Status
```bash
apple status --socket apple.sock
```

### 4. Inspect Hermetic Audit Logs
```bash
apple audit build_target_01
```

---

## 📄 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
