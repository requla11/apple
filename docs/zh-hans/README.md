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
│  (Readonly Jails & Overlay)  │  (Zero-Trust Offline Mirror) │
├──────────────────────────────┼──────────────────────────────┤
│  Process Isolation Runner    │  Real-Time Violation Monitor │
│  (Job Objects & Namespaces)  │  (IO Auditing & Telemetry)   │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ 核心特性

1. **封闭文件系统隔离 (`apple::isolation::fs`)**:
   * 以只读模式挂载源代码，将临时文件写入重定向至隔离目录，防止污染工作区。

2. **零信任网络阻断 (`apple::isolation::net`)**:
   * 编译期间全面阻断未授权外网请求，确保构建产物完全可重现。

3. **OS 原生内核级进程隔离 (`apple::isolation::process`)**:
   * **Windows**: 深度集成 Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，强限制内存)。
   * **Unix / Linux**: 采用进程组隔离（`setpgid`）与严格超时管理。

4. **环境熵清洗 (`apple::isolation::env`)**:
   * 剔除易变环境变量，保留核心构建参数。

5. **实时越界监控与审计报告 (`apple::monitor`, `apple::audit`)**:
   * 监控进程 I/O，即时捕捉未经声明的异常文件访问并生成审计报告。

---

## 🚀 命令行参考 (CLI)

```bash
# 在封闭沙箱中直接运行命令
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release

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
