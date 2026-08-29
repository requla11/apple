# 🍎 Apple: Fish 密閉沙箱與進程隔離守護進程

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](README.md)
>
> 🗺️ **[檢視完整技術路線圖 (ROADMAP)](ROADMAP.md)**


---

## 🎯 概述

**Apple** 是一個高效能的進程級密閉沙箱與隔離守護進程，作為 [Fish](https://github.com/requla11/fish) 建置編排引擎的底層執行屏障。在 Fish 負責依賴 DAG 圖、快取與並行調度的同時，Apple 將各個建置命令封裝在嚴密受控的環境中：硬連結工作區 Jail、精簡環境變數、多工具鏈離線策略、作業系統級隔離（Linux Namespaces、cgroups v2、seccomp-bpf、Windows Job Objects / 受限權杖 / AppContainer，以及 macOS Seatbelt SBPL），並提供即時的 I/O 違規與機密探測攔截。

Apple 提供 Rust 原生庫（供 `fish-sandbox` / `fish-executor` 呼叫）以及獨立的 CLI/守護進程二進位檔。

> **名稱說明：** "Apple" 是 Fish 🐟 的伴生專案代號。本專案為獨立開源工具，**與 Apple Inc. 無任何關聯、背書或贊助關係。**

---

## ⚡ 核心隔離特性

1. **🐧 Linux 深度核心隔離 (`apple::isolation::linux`)**:
   * **Linux 命名空間**: 非特權容器隔離 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
   * **cgroups v2 控制器**: 在 `/sys/fs/cgroup/apple_sandbox/{task_id}` 下精準控制記憶體限額 (`memory.max`)、CPU 配額 (`cpu.max`) 和核心親和性 (`cpuset.cpus`)。
   * **seccomp-bpf 過濾**: 過濾危險系統呼叫（`ptrace`、離線狀態下的原始通訊端綁定、核心模組載入等）。

2. **🪟 Windows 安全與 Job Objects (`apple::isolation::windows` & `apple::isolation::process`)**:
   * **Job Objects**: 硬體限制 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) 以及透過 `QueryInformationJobObject` 精準統計峰值記憶體。
   * **受限權杖與低完整性級別**: 剝離管理員權限並降低權杖至低完整性級別 (`SECURITY_MANDATORY_LOW_RID`)。
   * **AppContainer 隔離**: 支援 Windows AppContainer 原生沙箱隔離。

3. **🍎 macOS Seatbelt 策略配置 (`apple::isolation::macos`)**:
   * **SBPL 沙箱策略語言**: 生成凍結檔案系統存取與進程執行的策略 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
   * 自動透過 `sandbox-exec` 包裝 `clang`、`swiftc` 和 `rustc` 等編譯器命令。

4. **🔍 即時 Live I/O 與機密探測攔截器 (`apple::isolation::interceptor` & `apple::monitor`)**:
   * 即時監測建置進程存取路徑。
   * 針對機密檔案（`.env`、`id_rsa`、`.aws/credentials`、`/etc/shadow`、`/root`）的探測立即產生違規警報。
   * 校驗輸入標頭檔與檔案是否在 DAG 掛載規則內聲明。

5. **硬連結鏡像沙箱 (`apple::isolation::fs`)**:
   * 透過硬連結將原始碼樹鏡像至獨立 Jail 目錄，支援跨檔案系統自動降級複製。

6. **11+ 語言工具鏈離線策略 (`apple::isolation::net`)**:
   * 注入嚴格的離線環境變數（Cargo、Go、pip、npm/yarn/pnpm、Maven、Gradle、.NET、Swift、Dart）。

7. **雙通道確定性重現驗證 (`apple::verifier`)**:
   * 在隔離環境中進行雙次建置，第二次引入干擾的時間與區域變數 (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`) 並校驗 BLAKE3 雜湊。

---

## 🚀 CLI 命令參考

### 1. 啟動 IPC 守護進程
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. 單次沙箱執行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 驗證建置輸出確定性
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. 檢視審計記錄
```bash
apple audit
```
