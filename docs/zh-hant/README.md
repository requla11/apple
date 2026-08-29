# 🍎 Apple: Fish 構建系統的密封沙箱與行程隔離守護程序

> 🌐 **多語言導航 / Language Navigation:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](README.md)
>
> 🗺️ **[檢視完整技術路線圖 (Roadmap)](ROADMAP.md)**

---

## 🎯 概述

**Apple** 是為 [Fish](https://github.com/requla11/fish) 構建編排系統及企業級獨立工具鏈設計的高效能行程級密封沙箱引擎與隔離守護程序。在 Fish 負責 DAG 相依圖與分散式快取的同時，Apple 將編譯器及工具鏈指令封裝在極具安全保障的受控環境中：核心級沙箱、寫入時複製 (CoW) 零複製儲存隔離、即時分塊串流 IPC、不可分割任務取消以及 SLSA v1.0 / SPDX / CycloneDX 供應鏈安全。

> **名稱說明：** "Apple" 是 Fish 🐟 的協同專案代號。本專案是一個獨立的開源工具，**與 Apple Inc. 沒有任何關聯、認可或贊助關係。**

---

## ⚡ 核心架構特性

### 1. 🐧 Linux 深度核心隔離 (`apple::isolation::linux`)
- **Linux 命名空間**: 無特權容器級隔離 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
- **cgroups v2 控制器**: 在 `/sys/fs/cgroup/apple_sandbox/{task_id}` 限制硬體配額：記憶體 (`memory.max`)、CPU 配額 (`cpu.max`) 及核心親和度 (`cpuset.cpus`)。
- **seccomp-bpf 過濾**: 系統呼叫級安全策略，阻斷危險呼叫 (`ptrace`、離線時建立原始通訊端、載入核心模組)。
- **Landlock LSM**: Linux 核心級路徑存取控制規則，實施細粒度讀寫管控。

### 2. 🪟 Windows 安全與作業物件 (`apple::isolation::windows`)
- **Job Objects**: 硬體限制 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) 及精準峰值記憶體統計。
- **Restricted Tokens 與低完整性**: 移除管理員權限並將權杖完整性降為 `SECURITY_MANDATORY_LOW_RID`。
- **AppContainer 設定檔**: 原生 Windows AppContainer 隔離支援。

### 3. 🍏 macOS Seatbelt 設定檔 (`apple::isolation::macos`)
- **沙箱設定語言 (SBPL)**: 動態產生密封設定 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
- 使用 `sandbox-exec` 直接封裝原生編譯器 (`clang`, `swiftc`, `rustc`)。

### 4. ⚡ 零複製儲存隔離 (`apple::isolation::cow` & `fs`)
- **寫入時複製區塊複製 (CoW Block Cloning)**: 硬體加速支援 APFS `clonefile(2)`、Linux `FICLONE` / `Btrfs` reflink 及 Windows FSCTL 區塊複製（附帶實體連結回退機制）。
- **差異產物同步 (Differential Sync)**: 自動比對中繼資料快照以擷取變動和新產生的構建產物。

### 5. 🌊 即時串流 IPC 與行程取消 (`apple::protocol` & `daemon`)
- **分塊串流傳輸**: 透過 Unix Domain Sockets / Windows Named Pipes 非同步非阻塞傳輸 stdout/stderr 資料塊 (4KB 緩衝區)。
- **行程組終結**: 透過 Unix `SIGKILL` 行程組及 Windows Job Object 關閉實現原子級即時取消。

### 6. 🔐 企業級供應鏈安全與 SLSA v1.0 (`apple::provenance`, `attestation`, `sbom`)
- **SLSA v1.0 來源溯源**: 產生符合 in-toto Statement v1 標準的構建來源中繼資料，內含 BLAKE3 雜湊值。
- **密碼學證明簽署 (Attestation)**: 使用帶金鑰的 BLAKE3 MAC 簽署並驗證證明封套。
- **自動 SBOM 產生**: 匯出國際標準 **SPDX 2.3** 與 **CycloneDX 1.5** 格式的軟體物料清單。

---

## 🚀 CLI 命令列使用說明

### 1. 啟動 IPC 守護程序
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. 單次沙箱執行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 雙重執行可重現構建驗證
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. 產生 SLSA v1.0 來源中繼資料
```bash
apple provenance --task-id task_123 --artifacts target/release/my_bin --output provenance.json
```

### 5. 匯出軟體物料清單 (SPDX 2.3 / CycloneDX 1.5)
```bash
apple sbom --format spdx --task-id task_123 --artifacts target/release/my_bin --output sbom.spdx.json
apple sbom --format cyclonedx --task-id task_123 --artifacts target/release/my_bin --output sbom.cdx.json
```

### 6. 簽署與驗證 Attestation 封套
```bash
# 簽署
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# 驗證
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --verify --envelope envelope.json
```
