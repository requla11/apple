# 🍎 Apple: 核心級全封閉沙箱與處理程序隔離常駐程式

> 🌐 **語言導航 / 语言导航:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](README.md)

---

## 🎯 概述

**Apple** 是專為多語言構建編排系統 [Fish](https://github.com/requla11/fish) 量身打造的核心級全封閉（Hermetic）沙箱與零信任處理程序隔離常駐程式。

在 Fish 統籌高並發依賴 DAG 與分散式快取的同時，**Apple** 作為特權系統管理者，確保每次編譯器呼叫都在完全密閉、零污染且 100% 可重現的環境中執行。

---

## ⚡ 核心特性

1. **封閉檔案系統隔離 (`apple::isolation::fs`)**:
   * 以唯讀模式掛載原始碼，將臨時檔案寫入重定向至隔離目錄，防止污染工作區。

2. **零信任網路阻斷 (`apple::isolation::net`)**:
   * 編譯期間全面阻斷未授權外網請求，確保構建產物完全可重現。

3. **環境熵清洗 (`apple::isolation::env`)**:
   * 剔除易變環境變數，保留核心構建參數。

4. **即時越界監控 (`apple::monitor`)**:
   * 監控處理程序 I/O，即時捕捉未經聲明的異常檔案存取。

---

## 🚀 快速開始

```bash
# 啟動 Apple 常駐程式
apple --scratch-dir .apple-scratch --socket apple.sock
```

---

## 📄 開源許可證

本專案基於 MIT 許可證開源。詳見 [LICENSE](../../LICENSE)。
