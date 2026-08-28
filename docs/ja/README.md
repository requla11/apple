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
│  (Readonly Jails & Overlay)  │  (Zero-Trust Offline Mirror) │
├──────────────────────────────┼──────────────────────────────┤
│  Process Isolation Runner    │  Real-Time Violation Monitor │
│  (Job Objects & Namespaces)  │  (IO Auditing & Telemetry)   │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ 主な機能

1. **完全密閉ファイルシステム (`apple::isolation::fs`)**:
   * ソースツリーを読み取り専用でマウントし、一時的な書き込みを破棄可能な領域へリダイレクトします。

2. **ゼロトラストネットワーク遮断 (`apple::isolation::net`)**:
   * ビルド中の不正な外部通信を遮断し、100% 再現可能なビルドアーティファクトを保証します。

3. **OS ネイティブカーネル分離 (`apple::isolation::process`)**:
   * **Windows**: Windows Job Objects を利用してゾンビプロセスの発生と過剰なメモリ消費を防止。
   * **Unix / Linux**: プロセスグループ分離 (`setpgid`) とタイムアウト管理を適用。

4. **環境変数サニタイズ (`apple::isolation::env`)**:
   * 非決定的な環境変数を排除しつつ、必要なコンパイラフラグを安全に維持します。

5. **監査ログ・違反検知 (`apple::monitor`, `apple::audit`)**:
   * プロセス I/O をリアルタイムに監視し、未宣言のヘッダーや一時ファイルへのアクセス違反を記録。

---

## 🚀 CLI リファレンス

```bash
# サンドボックス内でコマンドを実行
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release

# デーモンを起動
apple daemon --scratch-dir .apple-scratch --socket apple.sock

# 状態を確認
apple status --socket apple.sock

# 監査レポートを表示
apple audit build_target_01
```

---

## 📄 ライセンス

MIT License のもとで公開されています。詳細は [LICENSE](../../LICENSE) をご覧ください。
