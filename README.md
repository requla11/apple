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
│  (Hard-Link CoW & Overlay)   │  (Zero-Trust Offline Mirror) │
├──────────────────────────────┼──────────────────────────────┤
│  Process Isolation Runner    │  Deterministic Verifier      │
│  (Job Objects & Namespaces)  │  (SLSA Level 3 Attestation)  │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Core Capabilities

1. **Hard-Link CoW Incremental Sandbox (`apple::isolation::fs`)**:
   * Mounts source trees via high-speed Hard-Link Farms (`mirror_hardlink_tree`) on Windows and Unix.
   * Redirects compiler writes into disposable scratch directories, preserving incremental build speed without mutating original source files.

2. **Zero-Trust Network Lockdown (`apple::isolation::net`)**:
   * Blackholes unauthorized outbound network requests during compilation tasks to guarantee 100% reproducible build artifacts.

3. **OS-Native Kernel Process Isolation (`apple::isolation::process`)**:
   * **Windows**: Implements Native Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, strict RAM ceilings).
   * **Unix / Linux**: Enforces process group leader isolation (`setpgid`) and timeout enforcement.

4. **SLSA Level 3 Deterministic Verifier (`apple::verifier`)**:
   * Executes dual-pass isolated builds with temporal perturbation (`SOURCE_DATE_EPOCH`, UTC timezone, clean env) and verifies bit-for-bit output determinism using BLAKE3 cryptographic hashing.

5. **Real-Time Violation Monitor & Audit Store (`apple::monitor`, `apple::audit`)**:
   * Inspects process I/O and immediately flags any compiler attempt to read un-declared headers or leaked build secrets.

---

## 🚀 CLI Reference

### 1. One-Shot Sandboxed Execution
```bash
# Run any command inside a hermetic offline jail with memory and timeout limits
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 2. Verify Bit-for-Bit Deterministic Reproducibility
```bash
# Verify build reproducibility under perturbed sandbox environments
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 3. Background Daemon Mode
```bash
# Start the IPC daemon for Fish build orchestration
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 4. Check Daemon Status
```bash
apple status --socket apple.sock
```

### 5. Inspect Hermetic Audit Logs
```bash
apple audit build_target_01
```

---

## 📄 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
