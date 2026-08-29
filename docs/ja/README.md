# 🍎 Apple: Fish ビルドシステム向け密閉サンドボックス＆プロセス分離デーモン

> 🌐 **言語ナビゲーション / Language Navigation:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)
>
> 🗺️ **[完全な技術ロードマップを表示 (Roadmap)](ROADMAP.md)**

---

## 🎯 概要

**Apple** は、[Fish](https://github.com/requla11/fish) ビルドオーケストレーションシステムおよび企業向けスタンドアロンツールチェーンのために設計された、超高速プロセスレベル密閉サンドボックスエンジンおよび分離デーモンです。Fish が DAG 依存関係と分散キャッシュを調整する一方で、Apple はコンパイラやツールチェーンのコマンドを完全に制御された隔離環境にカプセル化します：カーネルレベルのサンドボックス、Copy-on-Write (CoW) ゼロコピーストレージ隔離、リアルタイムチャンクストリーミング IPC、不可分タスクキャンセル、および SLSA v1.0 / SPDX / CycloneDX サプライチェーンセキュリティ。

> **名称に関する注意:** 「Apple」は Fish 🐟 のコンパニオンプロジェクト名です。本プロジェクトは独立したオープンソースツールであり、**Apple Inc. との提携、承認、スポンサーシップは一切ありません。**

---

## ⚡ コアアーキテクチャ機能

### 1. 🐧 Linux ディープカーネル分離 (`apple::isolation::linux`)
- **Linux Namespaces**: 非特権コンテナ分離 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
- **cgroups v2 コントローラー**: `/sys/fs/cgroup/apple_sandbox/{task_id}` 配下でハードウェア制限：メモリ (`memory.max`)、CPU クォータ (`cpu.max`)、CPU コアアフィニティ (`cpuset.cpus`)。
- **seccomp-bpf フィルター**: 危険なシステムコールをブロックするセキュリティポリシー (`ptrace`、オフライン時の raw ソケット作成、カーネルモジュールのロード)。
- **Landlock LSM**: Linux カーネルレベルのパス制御ルールによる詳細な読み取り/書き込み権限管理。

### 2. 🪟 Windows セキュリティとジョブオブジェクト (`apple::isolation::windows`)
- **Job Objects**: ハードウェア制限 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) と高精度なピークメモリ計測。
- **Restricted Tokens & Low Integrity**: 管理者特権を削除し、整合性レベルを `SECURITY_MANDATORY_LOW_RID` に引き下げ。
- **AppContainer プロファイル**: Windows ネイティブの AppContainer サンドボックスをサポート。

### 3. 🍏 macOS Seatbelt プロファイル (`apple::isolation::macos`)
- **Sandbox Profile Language (SBPL)**: 密閉プロファイルを動的生成 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
- ネイティブコンパイラ (`clang`, `swiftc`, `rustc`) を `sandbox-exec` で直接ラッピング。

### 4. ⚡ ゼロコピーストレージ隔離 (`apple::isolation::cow` & `fs`)
- **Copy-on-Write ブロッククローニング**: APFS `clonefile(2)`、Linux `FICLONE` / `Btrfs` reflink、および Windows FSCTL ブロッククローン（ハードリンクへのフォールバック付き）によるハードウェアアクセラレーション。
- **差分アーティファクト同期 (Differential Sync)**: メタデータスナップショットの自動比較により、変更されたファイルや新しく生成されたビルド成果物を高速抽出。

### 5. 🌊 リアルタイムストリーミング IPC とタスクキャンセル (`apple::protocol` & `daemon`)
- **チャンクストリーミング**: Unix Domain Sockets / Windows Named Pipes を介した非同期ノンブロッキング stdout/stderr ストリーミング (4KB バッファ)。
- **プロセスグループ終了**: Unix `SIGKILL` プロセスグループおよび Windows Job Object のクローズによる即時かつ確実なプロセス終了。

### 6. 🔐 エンタープライズサプライチェーンセキュリティ & SLSA v1.0 (`apple::provenance`, `attestation`, `sbom`)
- **SLSA v1.0 プロベナンス**: BLAKE3 ハッシュを含む in-toto Statement v1 準拠のビルド来歴メタデータを生成。
- **暗号アテステーション署名**: 秘密鍵付き BLAKE3 MAC によるアテステーションエンベロープの署名と検証。
- **自動 SBOM 生成**: 国際標準 **SPDX 2.3** および **CycloneDX 1.5** 形式のソフトウェア部品構成表を出力。

---

## 🚀 CLI コマンドリファレンス

### 1. IPC デーモンの起動
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. サンドボックス内でのワンショット実行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 再現可能ビルドの検証 (Dual-Pass Verification)
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. SLSA v1.0 来歴メタデータの生成
```bash
apple provenance --task-id task_123 --artifacts target/release/my_bin --output provenance.json
```

### 5. ソフトウェア部品構成表の出力 (SPDX 2.3 / CycloneDX 1.5)
```bash
apple sbom --format spdx --task-id task_123 --artifacts target/release/my_bin --output sbom.spdx.json
apple sbom --format cyclonedx --task-id task_123 --artifacts target/release/my_bin --output sbom.cdx.json
```

### 6. アテステーションエンベロープの署名と検証
```bash
# 署名
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# 検証
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --verify --envelope envelope.json
```
