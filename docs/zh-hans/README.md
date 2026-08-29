# 🍎 Apple:Fish 的 Hermetic 沙箱与进程隔离守护进程

> 🌐 **文档语言导航:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | **简体中文** | [繁體中文](../zh-hant/README.md)

---

## 🎯 概述

**Apple** 是一个进程级的 hermetic 沙箱与隔离守护进程,用于补充 [Fish](https://github.com/requla11/fish) 构建编排引擎。Fish 负责依赖图、缓存和并行调度,而 Apple 将单个构建命令包装在受控环境中:经过清洗的环境变量、临时工作副本、工具链级离线标志以及强制超时(Windows 上还包括 Job Object)。

Apple 既作为 Rust 库(由 `fish-sandbox` 使用)提供,也作为独立的 CLI/守护进程提供。

> **关于名称:** "Apple" 是 Fish 🐟 的姊妹项目名称。本项目是一个独立的开源工具,**与 Apple Inc. 无关,未获得其认可或赞助。**

## ⚡ Apple 实际做什么

1. **硬链接镜像沙箱 (`apple::isolation::fs`)**:
   * 使用硬链接(跨文件系统时自动回退为复制)将源码树镜像到每个任务的 jail 目录中。
   * 编译器的写入落在 jail 内,原始源码树保持不变。

2. **环境变量清洗 (`apple::isolation::env`)**:
   * 去除允许列表(以及 `FISH_*` 和 `APPLE_*` 前缀)之外的所有环境变量,并将 `TMPDIR`/`TEMP`/`TMP` 指向 jail。

3. **尽力而为的网络抑制 (`apple::isolation::net`)**:
   * 注入 Cargo、Go、pip、npm 会遵循的黑洞代理变量和离线标志(`CARGO_NET_OFFLINE`、`GOPROXY=off` 等)。
   * **这不是防火墙。** 忽略代理变量的进程仍然可以访问网络。未实现内核级强制执行(网络命名空间)。

4. **进程隔离 (`apple::isolation::process`)**:
   * **Windows**: 具有 `KILL_ON_JOB_CLOSE` 和可选内存上限的 Job Object;子进程使用 `CREATE_NO_WINDOW`。
   * **Unix**: 基于 `setpgid` 的进程组隔离和硬超时。
   * 这是用户空间的进程隔离 — 不使用 namespace、seccomp 或 AppContainer。

5. **双趟确定性检查 (`apple::verifier`)**:
   * 在全新的 jail 中运行相同的构建两次;第二趟使用被扰动的区域设置/时间变量(`SOURCE_DATE_EPOCH`、`TZ`、`LC_ALL`)。
   * 比较工件的 BLAKE3 哈希。这是自我声明的可重现性检查,**不是** SLSA 认证。

6. **审计记录 (`apple::audit`)**:
   * 守护进程将执行结果(退出码、耗时、违规)以 JSON 形式持久化到 `<scratch>/audit/<task_id>.json`,供 CLI 查阅。

7. **违规检查 (`apple::monitor`)**:
   * 基于路径前缀的策略检查器,仅以库的形式提供。未接入实时 syscall/进程 I/O 拦截。

## 🚀 CLI 参考

### 1. 启动 IPC 守护进程
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```
通过 Unix socket 或 Windows 命名管道提供以换行符分隔的 JSON(`DaemonMessage`),收到 `Shutdown` 消息或按下 Ctrl+C 后退出。

### 2. 单次沙箱执行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 验证输出的确定性
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```
要求构建在 jail **内部**生成工件,以便两趟都能计算哈希。

### 4. 查看守护进程状态
```bash
apple status --socket apple.sock
```
通过 IPC 真实地 ping 守护进程,报告可达性、版本和活动沙箱数量。

### 5. 查看审计记录
```bash
apple audit <task_id>
apple telemetry <task_id>
```
读取守护进程先前写入的 JSON 记录。如果记录不存在,CLI 会如实报告 — 绝不打印占位数字。

### 6. 自动检测语言配置
```bash
apple profile-detect --dir .
```

## 🧪 已知限制

* 无内核级沙箱(Linux 无 namespace/seccomp,Windows 无 AppContainer/AppLocker)。
* 网络封锁是基于环境变量的建议性措施,而非强制执行。
* 违规检查器是仅限库使用的路径检查器,不是运行时 I/O 拦截器。
* 峰值内存和 CPU 时间采样未实现;遥测只报告运行器真正掌握的信息(退出码、耗时)。
* 确定性验证器要求工件在 jail 内生成;无法对写入沙箱之外的工件计算哈希。
* IPC 仅限单机(Unix socket / 命名管道)。

## 📄 许可证与免责声明

基于 MIT 许可证发布。详情请参阅 [LICENSE](../../LICENSE)。

> **免责声明:** 本项目是一个独立的开源工具,与 Apple Inc. 无关,未获得其认可或赞助。
