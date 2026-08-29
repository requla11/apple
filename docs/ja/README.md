# 🍎 Apple: カーネルレベル完全密閉サンドボックス＆プロセス分離デーモン

> 🌐 **言語ナビゲーション:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 概要

**Apple** は、多言語ビルドオーケストレーションシステム [Fish](https://github.com/requla11/fish) を補完するために開発された、カーネルレベルの密閉型（Hermetic）サンドボックスおよびゼロトラストプロセス分離デーモンです。

Fish が依存関係 DAG、CAS キャッシュ、投機的コンパイルを制御する一方で、**Apple** はシステムハイパーバイザーとして機能し、すべてのコンパイラ呼び出しが完全に密閉され、副作用のない再現可能な環境で実行されることを保証します。

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

## ⚡ 主な機能

1. **高速 Hard-Link CoW インクリメンタルサンドボックス (`apple::isolation::fs`)**:
   * ハードリンクファーム構造 (`mirror_hardlink_tree`) を採用し、元のソースを汚染することなく高速な増分ビルド性能を維持。

2. **ゼロトラストネットワーク遮断 (`apple::isolation::net`)**:
   * ビルド中の不正な外部通信を遮断し、100% 再現可能なビルドアーティファクトを保証します。

3. **OS ネイティブカーネル分離 (`apple::isolation::process`)**:
   * **Windows**: Windows Job Objects を利用してゾンビプロセスの発生と過剰なメモリ消費を防止。
   * **Unix / Linux**: プロセスグループ分離 (`setpgid`) とタイムアウト管理を適用。

4. **SLSA Level 3 決定性検証エンジン (`apple::verifier`)**:
   * タイムスタンプや環境変数を変動させた 2 段階の独立ビルドを実行し、BLAKE3 ハッシュにより完全な再現性を検証。

5. **監査ログ・違反検知 (`apple::monitor`, `apple::audit`)**:
   * プロセス I/O をリアルタイムに監視し、未宣言のヘッダーや一時ファイルへのアクセス違反を記録。

---

## 🚀 CLI リファレンス

```bash
# サンドボックス内でコマンドを実行
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release

# 決定性と再現性を検証
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release

# デーモンを起動
apple daemon --scratch-dir .apple-scratch --socket apple.sock

# 状態を確認
apple status --socket apple.sock

# 監査レポートを表示
apple audit build_target_01
```

---

## 📄 ライセンスと免責事項

MIT License のもとで公開されています。詳細は [LICENSE](../../LICENSE) をご覧ください。

> **免責事項:** 本プロジェクトは独立したオープンソースツールであり、Apple Inc. との提携、承認、後援を受けているものではありません。
