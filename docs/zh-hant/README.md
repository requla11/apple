# 🍎 Apple: 核心級全封閉沙箱與處理程序隔離常駐程式

> 🌐 **語言導航 / 语言导航:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](README.md)

---

## 🎯 概述

**Apple** 是專為多語言構建編排系統 [Fish](https://github.com/requla11/fish) 量身打造的核心級全封閉（Hermetic）沙箱與零信任處理程序隔離常駐程式。

在 Fish 統籌高並發依賴 DAG 與分散式快取的同時，**Apple** 作為特權系統管理者，確保每次編譯器呼叫都在完全密閉、零污染且 100% 可重現的環境中執行。

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
   * 基於硬連結農場機制 (`mirror_hardlink_tree`)，在杜絕污染原始碼的前提下保留極致的增量編譯效能。

2. **零信任網路阻斷 (`apple::isolation::net`)**:
   * 編譯期間全面阻斷未授權外網請求，確保構建產物完全可重現。

3. **OS 原生核心級處理程序隔離 (`apple::isolation::process`)**:
   * **Windows**: 深度整合 Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，嚴格限制記憶體)。
   * **Unix / Linux**: 採用處理程序群組隔離（`setpgid`）與嚴格逾時管理。

4. **SLSA Level 3 確定性構建驗證器 (`apple::verifier`)**:
   * 雙通道攝動驗證（`SOURCE_DATE_EPOCH`、UTC 時區、純淨環境），透過 BLAKE3 密碼學雜湊比對確保 100% 逐位可重現。

5. **即時越界監控與審計報告 (`apple::monitor`, `apple::audit`)**:
   * 監控處理程序 I/O，即時捕捉未經聲明的異常檔案存取並生成審計報告。

---

## 🚀 命令列參考 (CLI)

```bash
# 在封閉沙箱中直接執行命令
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release

# 驗證構建確定性與可重現性 (SLSA Level 3)
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release

# 啟動後台常駐程式
apple daemon --scratch-dir .apple-scratch --socket apple.sock

# 檢查常駐程式狀態
apple status --socket apple.sock

# 查看審計報告
apple audit build_target_01
```

---

## 📄 開源許可證

本專案基於 MIT 許可證開源。詳見 [LICENSE](../../LICENSE)。
