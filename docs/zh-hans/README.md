# 🍎 Apple: 内核级全封闭沙箱与进程隔离守护进程

> 🌐 **语言导航 / 語言導航:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 概述

**Apple** 是专为多语言构建编排系统 [Fish](https://github.com/requla11/fish) 量身定制的内核级全封闭（Hermetic）沙箱与零信任进程隔离守护进程。

在 Fish 统筹高并发依赖 DAG 与分布式缓存的同时，**Apple** 作为特权系统管理者，确保每次编译器调用都在完全密闭、零污染且 100% 可重现的环境中执行。

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

## ⚡ 核心特性

1. **高效 Hard-Link CoW 增量沙箱 (`apple::isolation::fs`)**:
   * 基于硬链接农场机制 (`mirror_hardlink_tree`)，在杜绝污染源码的前提下保留极致的增量编译性能。

2. **零信任网络阻断 (`apple::isolation::net`)**:
   * 编译期间全面阻断未授权外网请求，确保构建产物完全可重现。

3. **OS 原生内核级进程隔离 (`apple::isolation::process`)**:
   * **Windows**: 深度集成 Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，强限制内存)。
   * **Unix / Linux**: 采用进程组隔离（`setpgid`）与严格超时管理。

4. **SLSA Level 3 确定性构建验证器 (`apple::verifier`)**:
   * 双通道摄动验证（`SOURCE_DATE_EPOCH`、UTC 时区、纯净环境），通过 BLAKE3 密码学哈希比对确保 100% 逐位可重现。

5. **实时越界监控与审计报告 (`apple::monitor`, `apple::audit`)**:
   * 监控进程 I/O，即时捕捉未经声明的异常文件访问并生成审计报告。

---

## 🚀 命令行参考 (CLI)

```bash
# 在封闭沙箱中直接运行命令
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release

# 验证构建确定性与可重现性 (SLSA Level 3)
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release

# 启动后台守护进程
apple daemon --scratch-dir .apple-scratch --socket apple.sock

# 检查守护进程状态
apple status --socket apple.sock

# 查看审计报告
apple audit build_target_01
```

---

## 📄 开源许可证

本项目基于 MIT 许可证开源。详见 [LICENSE](../../LICENSE)。
