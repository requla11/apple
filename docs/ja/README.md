# 🍎 Apple: Fish 用 Hermetic サンドボックス & プロセス分離デーモン

> 🌐 **ドキュメント言語ナビゲーション:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | **日本語** | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 概要

**Apple** は、[Fish](https://github.com/requla11/fish) ビルドオーケストレーションエンジンを補完する、プロセスレベルの hermetic サンドボックスおよび分離デーモンです。Fish が依存関係グラフ、キャッシュ、並列スケジューリングを担当する一方、Apple は個々のビルドコマンドを管理された環境でラップします: スクラブされた環境変数、スクラッチ作業コピー、ツールチェーンレベルのオフラインフラグ、強制タイムアウト(Windows では Job Object)。

Apple は Rust ライブラリ(`fish-sandbox` から利用)とスタンドアロンの CLI/デーモンの両方として提供されます。

> **名前について:** "Apple" は Fish 🐟 の姉妹プロジェクト名です。本プロジェクトは独立したオープンソースツールであり、**Apple Inc. とは無関係であり、承認・後援されていません。**

## ⚡ Apple が実際にしていること

1. **ハードリンクミラーサンドボックス (`apple::isolation::fs`)**:
   * ハードリンク(ファイルシステムを跨ぐ場合は自動でコピーにフォールバック)により、ソースツリーをタスクごとの jail ディレクトリにミラーリングします。
   * コンパイラの書き込みは jail 内に収まり、元のツリーは変更されません。

2. **環境変数のスクラブ (`apple::isolation::env`)**:
   * 許可リスト(および `FISH_*`・`APPLE_*` 接頭辞)以外の環境変数をすべて除去し、`TMPDIR`/`TEMP`/`TMP` を jail に向けます。

3. **ベストエフォートのネットワーク抑制 (`apple::isolation::net`)**:
   * Cargo・Go・pip・npm が解釈するブラックホールプロキシ変数とオフラインフラグ(`CARGO_NET_OFFLINE`、`GOPROXY=off` など)を注入します。
   * **これはファイアウォールではありません。** プロキシ変数を無視するプロセスは依然としてネットワークにアクセスできます。カーネルレベルの強制(ネットワーク名前空間)は実装されていません。

4. **プロセス分離 (`apple::isolation::process`)**:
   * **Windows**: `KILL_ON_JOB_CLOSE` とオプションのメモリ上限を持つ Job Object、子プロセスには `CREATE_NO_WINDOW`。
   * **Unix**: `setpgid` によるプロセスグループ分離とハードタイムアウト。
   * これはユーザースペースのプロセス分離であり、名前空間・seccomp・AppContainer は使用していません。

5. **2 パス決定論チェック (`apple::verifier`)**:
   * 同じビルドを新鮮な jail 内で 2 回実行します。2 パス目はロケール・時刻変数(`SOURCE_DATE_EPOCH`、`TZ`、`LC_ALL`)を変えて実行します。
   * アーティファクトの BLAKE3 ハッシュを比較します。これは自己宣言型の再現性チェックであり、**SLSA 証明ではありません。**

6. **監査レコード (`apple::audit`)**:
   * デーモンは実行結果(終了コード、所要時間、違反)を `<scratch>/audit/<task_id>.json` に JSON として永続化し、CLI から確認できます。

7. **違反チェック (`apple::monitor`)**:
   * パスプレフィックスポリシーチェッカー。ライブラリとして利用可能ですが、ライブの syscall/プロセス I/O 傍受には接続されていません。

## 🚀 CLI リファレンス

### 1. IPC デーモンの起動
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```
Unix ソケットまたは Windows 名前付きパイプ上で、改行区切り JSON(`DaemonMessage`)を提供します。`Shutdown` メッセージまたは Ctrl+C で終了します。

### 2. 1 回限りのサンドボックス実行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 出力の決定論検証
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```
両パスでハッシュできるよう、ビルドは jail **内**にアーティファクトを生成する必要があります。

### 4. デーモンの状態確認
```bash
apple status --socket apple.sock
```
実際のデーモンに IPC 経由で ping を送り、到達可能性・バージョン・アクティブなサンドボックス数を報告します。

### 5. 監査レコードの閲覧
```bash
apple audit <task_id>
apple telemetry <task_id>
```
デーモンが書き込んだ JSON レコードを読み取ります。レコードが存在しない場合はその旨を報告します — プレースホルダーの数値を表示することはありません。

### 6. 言語プロファイルの自動検出
```bash
apple profile-detect --dir .
```

## 🧪 既知の制限事項

* カーネルレベルのサンドボックス化は未実装(Linux の namespace/seccomp、Windows の AppContainer/AppLocker なし)。
* ネットワーク遮断は ENV 変数ベースの助言的なもので、強制ではありません。
* 違反モニターはライブラリ専用のパスチェッカーであり、ランタイム I/O インターセプターではありません。
* ピークメモリと CPU 時間のサンプリングは未実装。テレメトリはランナーが実際に把握している情報(終了コード、所要時間)のみを報告します。
* 決定論検証器は、アーティファクトが jail 内で生成されることを要求します。
* IPC は単一ホスト限定(Unix ソケット / 名前付きパイプ)。

## 📄 ライセンス & 免責事項

MIT ライセンスで提供されています。詳細は [LICENSE](../../LICENSE) を参照してください。

> **免責事項:** 本プロジェクトは独立したオープンソースツールであり、Apple Inc. とは無関係であり、承認・後援されていません。
