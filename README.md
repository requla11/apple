# 🍎 Apple: Hermetic Sandbox & Process Isolation Daemon for Fish

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](README.md) | [Tiếng Việt](docs/vi/README.md) | [日本語](docs/ja/README.md) | [简体中文](docs/zh-hans/README.md) | [繁體中文](docs/zh-hant/README.md)
>
> 🗺️ **[View Full Technical Roadmap](ROADMAP.md)**


---

## 🎯 Overview

**Apple** is a high-performance, process-level hermetic sandbox and isolation daemon that complements the [Fish](https://github.com/requla11/fish) build orchestration engine. While Fish coordinates dependency graphs, caching, and parallel scheduling, Apple wraps individual build commands in a strictly controlled environment: hardlinked workspace jails, scrubbed environment sets, multi-toolchain network offline policies, OS-level containment (Linux Namespaces, cgroups v2, seccomp-bpf, Windows Job Objects / Restricted Tokens, and macOS Seatbelt SBPL), and real-time live I/O violation interception.

Apple ships as both a Rust library (consumed by `fish-sandbox` / `fish-executor`) and a standalone CLI/daemon.

> **Note on the name:** "Apple" is a companion project name for Fish 🐟. This project is an independent open-source tool and is **not affiliated with, endorsed, or sponsored by Apple Inc.**

---

## ⚡ Core Isolation Capabilities

1. **🐧 Deep Linux Kernel Isolation (`apple::isolation::linux`)**:
   * **Linux Namespaces**: Unprivileged container isolation (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`).
   * **cgroups v2 Controller**: Hardware resource quotas under `/sys/fs/cgroup/apple_sandbox/{task_id}` for RAM (`memory.max`), CPU quota (`cpu.max`), and core affinity (`cpuset.cpus`).
   * **seccomp-bpf Filter**: System call policy filtering blocking unauthorized syscalls (`ptrace`, raw socket bindings when offline, kernel module operations).

2. **🪟 Windows Security & Job Objects (`apple::isolation::windows` & `apple::isolation::process`)**:
   * **Job Objects**: Hardware limits (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) and exact peak memory accounting via `QueryInformationJobObject`.
   * **Restricted Tokens & Low Integrity**: Strips administrator privileges and drops token integrity to Low Integrity (`SECURITY_MANDATORY_LOW_RID`).
   * **AppContainer Profiles**: Native Windows AppContainer sandboxing support.

3. **🍎 macOS Seatbelt Profiles (`apple::isolation::macos`)**:
   * **SBPL (Sandbox Profile Language)**: Generates hermetic sandbox profiles freezing filesystem access and process executions (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`).
   * Seamless wrapping with `sandbox-exec` for compilers like `clang`, `swiftc`, and `rustc`.

4. **🔍 Real-Time Live I/O & Secret Interceptor (`apple::isolation::interceptor` & `apple::monitor`)**:
   * Inspects accessed paths in real-time.
   * Immediately flags probes against protected secret patterns (`.env`, `id_rsa`, `.aws/credentials`, `/etc/shadow`, `/root`).
   * Verifies that compiler inputs match declared DAG input mount rules.

5. **Hard-link Mirror Sandbox (`apple::isolation::fs`)**:
   * Mirrors source trees into per-task jail folders with hardlinks and automatic cross-filesystem fallback.

6. **Toolchain-Level Offline Policy (`apple::isolation::net`)**:
   * Injects strict offline environment flags across 11+ languages (Cargo, Go, pip, npm/yarn/pnpm, Maven, Gradle, .NET, Swift, Dart).

7. **Dual-Pass Determinism Verifier (`apple::verifier`)**:
   * Executes dual-pass reproducible builds with perturbed timestamps and locales (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`) and BLAKE3 hash auditing.

---

## 🚀 CLI Reference

### 1. Start the IPC daemon
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. One-shot sandboxed execution
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Verify deterministic output
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. Inspect audit logs
```bash
apple audit
```
