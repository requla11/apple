# 🍎 Apple:Fish 的 Hermetic 沙箱與程序隔離守護程序

> 🌐 **文件語言導覽:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | **繁體中文**

---

## 🎯 概述

**Apple** 是一個程序級的 hermetic 沙箱與隔離守護程序,用於補充 [Fish](https://github.com/requla11/fish) 建置編排引擎。Fish 負責依賴圖、快取和並行排程,而 Apple 將單個建置命令包裝在受控環境中:經過清洗的環境變數、臨時工作副本、工具鏈級離線旗標以及強制逾時(Windows 上還包括 Job Object)。

Apple 既作為 Rust 函式庫(由 `fish-sandbox` 使用)提供,也作為獨立的 CLI/守護程序提供。

> **關於名稱:** "Apple" 是 Fish 🐟 的姊妹專案名稱。本專案是一個獨立的開源工具,**與 Apple Inc. 無關,未獲得其認可或贊助。**

## ⚡ Apple 實際做什麼

1. **硬連結映像沙箱 (`apple::isolation::fs`)**:
   * 使用硬連結(跨檔案系統時自動回退為複製)將原始碼樹映像到每個任務的 jail 目錄中。
   * 編譯器的寫入落在 jail 內,原始原始碼樹保持不變。

2. **環境變數清洗 (`apple::isolation::env`)**:
   * 移除允許清單(以及 `FISH_*` 和 `APPLE_*` 前綴)之外的所有環境變數,並將 `TMPDIR`/`TEMP`/`TMP` 指向 jail。

3. **盡力而為的網路抑制 (`apple::isolation::net`)**:
   * 注入 Cargo、Go、pip、npm 會遵循的黑洞代理變數和離線旗標(`CARGO_NET_OFFLINE`、`GOPROXY=off` 等)。
   * **這不是防火牆。** 忽略代理變數的程序仍然可以存取網路。未實作核心級強制執行(網路命名空間)。

4. **程序隔離 (`apple::isolation::process`)**:
   * **Windows**: 具有 `KILL_ON_JOB_CLOSE` 和可選記憶體上限的 Job Object;子程序使用 `CREATE_NO_WINDOW`。
   * **Unix**: 基於 `setpgid` 的程序群組隔離和硬逾時。
   * 這是使用者空間的程序隔離 — 不使用 namespace、seccomp 或 AppContainer。

5. **雙趟確定性檢查 (`apple::verifier`)**:
   * 在全新的 jail 中執行相同的建置兩次;第二趟使用被擾動的語系/時間變數(`SOURCE_DATE_EPOCH`、`TZ`、`LC_ALL`)。
   * 比較工件的 BLAKE3 雜湊。這是自我聲明的可重現性檢查,**不是** SLSA 認證。

6. **稽核記錄 (`apple::audit`)**:
   * 守護程序將執行結果(結束代碼、耗時、違規)以 JSON 形式持久化到 `<scratch>/audit/<task_id>.json`,供 CLI 查閱。

7. **違規檢查 (`apple::monitor`)**:
   * 基於路徑前綴的原則檢查器,僅以函式庫的形式提供。未接入即時 syscall/程序 I/O 攔截。

## 🚀 CLI 參考

### 1. 啟動 IPC 守護程序
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```
透過 Unix socket 或 Windows 具名管道提供以換行符分隔的 JSON(`DaemonMessage`),收到 `Shutdown` 訊息或按下 Ctrl+C 後退出。

### 2. 單次沙箱執行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 驗證輸出的確定性
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```
要求建置在 jail **內部**產生工件,以便兩趟都能計算雜湊。

### 4. 查看守護程序狀態
```bash
apple status --socket apple.sock
```
透過 IPC 真實地 ping 守護程序,報告可達性、版本和活動沙箱數量。

### 5. 查看稽核記錄
```bash
apple audit <task_id>
apple telemetry <task_id>
```
讀取守護程序先前寫入的 JSON 記錄。如果記錄不存在,CLI 會如實報告 — 絕不印出佔位數字。

### 6. 自動偵測語言設定檔
```bash
apple profile-detect --dir .
```

## 🧪 已知限制

* 無核心級沙箱(Linux 無 namespace/seccomp,Windows 無 AppContainer/AppLocker)。
* 網路封鎖是基於環境變數的建議性措施,而非強制執行。
* 違規檢查器是僅限函式庫使用的路徑檢查器,不是執行時 I/O 攔截器。
* 峰值記憶體和 CPU 時間取樣未實作;遙測只報告執行器真正掌握的資訊(結束代碼、耗時)。
* 確定性驗證器要求工件在 jail 內產生;無法對寫入沙箱之外的工件計算雜湊。
* IPC 僅限單機(Unix socket / 具名管道)。

## 📄 授權條款與免責聲明

基於 MIT 授權條款發布。詳情請參閱 [LICENSE](../../LICENSE)。

> **免責聲明:** 本專案是一個獨立的開源工具,與 Apple Inc. 無關,未獲得其認可或贊助。
