# 🍎 Apple: Hermetic Sandbox & Process Isolation Daemon for Fish

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](README.md) | [Tiếng Việt](docs/vi/README.md) | [日本語](docs/ja/README.md) | [简体中文](docs/zh-hans/README.md) | [繁體中文](docs/zh-hant/README.md)
>
> 🗺️ **[View Full Technical Roadmap](ROADMAP.md)**

---

## 🎯 Overview

**Apple** is an ultra-fast, hermetic sandbox engine and isolation daemon built for the [Fish](https://github.com/requla11/fish) build orchestration system and standalone enterprise toolchains. While Fish coordinates DAG dependencies and distributed caching, Apple wraps compiler and toolchain commands in a strictly contained environment: kernel-level sandboxing, Copy-on-Write (CoW) zero-copy storage jails, real-time chunked streaming IPC, cryptographic process cancellation, and SLSA v1.0 / SPDX / CycloneDX supply chain security.

> **Note on the name:** "Apple" is a companion project name for Fish 🐟. This project is an independent open-source tool and is **not affiliated with, endorsed, or sponsored by Apple Inc.**

---

## ⚡ Key Architectural Capabilities

### 1. 🐧 Deep Kernel Isolation (`apple::isolation::linux`)
- **Linux Namespaces**: Unprivileged container isolation (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`).
- **cgroups v2 Controller**: Hardware resource quotas under `/sys/fs/cgroup/apple_sandbox/{task_id}` for RAM (`memory.max`), CPU quota (`cpu.max`), and core affinity (`cpuset.cpus`).
- **seccomp-bpf Filter**: System call policy filtering blocking unauthorized syscalls (`ptrace`, raw socket bindings when offline, kernel module operations).
- **Landlock LSM**: Linux kernel-enforced path restriction rules granting fine-grained read/write access.

### 2. 🪟 Windows Security & Job Objects (`apple::isolation::windows`)
- **Job Objects**: Hardware limits (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) and exact peak memory accounting via `QueryInformationJobObject`.
- **Restricted Tokens & Low Integrity**: Drops administrator privileges and lowers integrity level to `SECURITY_MANDATORY_LOW_RID`.
- **AppContainer Profiles**: Native Windows AppContainer sandboxing support.

### 3. 🍏 macOS Seatbelt Profiles (`apple::isolation::macos`)
- **Sandbox Profile Language (SBPL)**: Dynamically generates hermetic profiles (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`).
- Direct wrapping with `sandbox-exec` for native toolchains (`clang`, `swiftc`, `rustc`).

### 4. ⚡ Zero-Copy Storage Jails (`apple::isolation::cow` & `fs`)
- **Copy-on-Write Block Cloning**: Hardware-accelerated APFS `clonefile(2)`, Linux `FICLONE` / `Btrfs` reflink, and Windows FSCTL block cloning with hardlink fallback.
- **Differential Artifact Sync**: Automatic metadata snapshot comparisons extracting modified and newly produced build artifacts.

### 5. 🌊 Real-Time Streaming IPC & Cancellation (`apple::protocol` & `daemon`)
- **Streaming Chunks**: Non-blocking async stdout/stderr chunk streaming (4KB buffers) over Unix Domain Sockets / Windows Named Pipes.
- **Process Group Termination**: Atomic cancellation via Unix `SIGKILL` process groups and Windows Job Object closing.

### 6. 🔐 Enterprise Supply Chain Security & SLSA v1.0 (`apple::provenance`, `attestation`, `sbom`)
- **SLSA v1.0 Provenance**: Generates in-toto Statement v1 metadata with cryptographic BLAKE3 input/output digests.
- **Cryptographic Attestation**: Signs and verifies attestation envelopes with keyed BLAKE3 MACs.
- **Automated SBOM**: Exports software bill of materials in standard **SPDX 2.3** and **CycloneDX 1.5** formats.

---

## 🚀 CLI Usage

### 1. Start the IPC Daemon
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. One-Shot Sandboxed Execution
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Dual-Pass Reproducible Build Verification
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. Generate SLSA v1.0 Provenance
```bash
apple provenance --task-id task_123 --artifacts target/release/my_bin --output provenance.json
```

### 5. Export SBOM (SPDX 2.3 / CycloneDX 1.5)
```bash
apple sbom --format spdx --task-id task_123 --artifacts target/release/my_bin --output sbom.spdx.json
apple sbom --format cyclonedx --task-id task_123 --artifacts target/release/my_bin --output sbom.cdx.json
```

### 6. Sign and Verify Attestation Envelopes
```bash
# Sign
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# Verify
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --verify --envelope envelope.json
```
