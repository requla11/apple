# 🍎 Apple: Fish 向け密閉サンドボックス＆プロセス分離デーモン

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 概要

**Apple** は、[Fish](https://github.com/requla11/fish) ビルドオーケストレーションエンジンを補完する高性能なプロセスレベルの密閉サンドボックスおよび分離デーモンです。Fish が依存関係 DAG、キャッシュ、並列スケジューリングを管理する一方で、Apple は各ビルドコマンドを厳密に制御された環境（ハードリンク Jail、クリーンアップされた環境変数、多言語オフラインポリシー、OS レベルの分離（Linux Namespaces、cgroups v2、seccomp-bpf、Windows Job Objects / 制限付きトークン / AppContainer、macOS Seatbelt SBPL）、リアルタイム I/O 違反検出）で実行します。

Apple は Rust ライブラリ（`fish-sandbox` / `fish-executor` から利用可能）およびスタンドアロンの CLI / デーモンとして提供されます。

> **名称について:** 「Apple」は Fish 🐟 のコンパニオンプロジェクト名です。本プロジェクトは独立したオープンソースソフトウェアであり、**Apple Inc. との関係、承認、後援はありません。**

---

## ⚡ コア分離機能

1. **🐧 Linux 深層カーネル分離 (`apple::isolation::linux`)**:
   * **Linux Namespaces**: 非特権コンテナ分離 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
   * **cgroups v2 コントローラー**: `/sys/fs/cgroup/apple_sandbox/{task_id}` によるメモリ上限 (`memory.max`)、CPU クォータ (`cpu.max`)、コアアフィニティ (`cpuset.cpus`) の厳格制御。
   * **seccomp-bpf フィルター**: 危険なシステムコールの遮断（`ptrace`、オフライン時の raw ソケットバインド、カーネルモジュール操作など）。

2. **🪟 Windows セキュリティ＆ Job Objects (`apple::isolation::windows` & `apple::isolation::process`)**:
   * **Job Objects**: ハードウェア制限 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) および `QueryInformationJobObject` による正確なピークメモリ計測。
   * **制限付きトークン＆ Low Integrity**: 管理者権限の剥奪と Low Integrity レベルへの降格 (`SECURITY_MANDATORY_LOW_RID`)。
   * **AppContainer 分離**: Windows AppContainer ネイティブサンドボックスのサポート。

3. **🍎 macOS Seatbelt プロファイル (`apple::isolation::macos`)**:
   * **SBPL (Sandbox Profile Language)**: ファイルシステムアクセスとプロセス実行を凍結するプロファイル生成 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
   * `sandbox-exec` による `clang`、`swiftc`、`rustc` などのコンパイラコマンドの透過的ラッピング。

4. **🔍 リアルタイム Live I/O ＆ 機密ファイル監査 (`apple::isolation::interceptor` & `apple::monitor`)**:
   * リアルタイムでのアクセスパス監査。
   * 機密ファイル（`.env`、`id_rsa`、`.aws/credentials`、`/etc/shadow`、`/root`）への不正プローブを即座に違反検知。
   * コンパイラの入力が DAG マウントルールに準拠しているかを検証。

5. **ハードリンクミラーサンドボックス (`apple::isolation::fs`)**:
   * ハードリンクを用いてタスク専用 Jail にソースツリーをミラーリング（ファイルシステムを跨ぐ場合は自動フォールバックコピー）。

6. **11 以上の言語向けオフラインポリシー (`apple::isolation::net`)**:
   * Cargo、Go、pip、npm/yarn/pnpm、Maven、Gradle、.NET、Swift、Dart に対する厳格なオフライン環境変数の注入。

7. **デュアルパス再現性検証 (`apple::verifier`)**:
   * タイムスタンプやロケール変数（`SOURCE_DATE_EPOCH`、`TZ`、`LC_ALL`）を摂動させた2回のビルドを実行し、BLAKE3 ハッシュを照合。

---

## 🚀 CLI リファレンス

### 1. IPC デーモンの起動
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. サンドボックス内での単発実行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. ビルド成果物の決定性検証
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. 監査ログの確認
```bash
apple audit
```
