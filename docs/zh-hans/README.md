# 🍎 Apple: Fish 密闭沙箱与进程隔离守护进程

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](README.md) | [繁體中文](../zh-hant/README.md)
>
> 🗺️ **[查看完整技术路线图 (ROADMAP)](ROADMAP.md)**


---

## 🎯 概述

**Apple** 是一个高性能的进程级密闭沙箱与隔离守护进程，作为 [Fish](https://github.com/requla11/fish) 构建编排引擎的底层执行屏障。在 Fish 负责依赖 DAG 图、缓存与并行调度的同时，Apple 将各个构建命令封装在严密受控的环境中：硬链接工作区 Jail、精简环境变量、多工具链离线策略、操作系统级隔离（Linux Namespaces、cgroups v2、seccomp-bpf、Windows Job Objects / 受限令牌 / AppContainer，以及 macOS Seatbelt SBPL），并提供实时的 I/O 违规与秘密探测拦截。

Apple 提供 Rust 原生库（供 `fish-sandbox` / `fish-executor` 调用）以及独立的 CLI/守护进程二进制。

> **名称说明：** "Apple" 是 Fish 🐟 的伴生项目代号。本项目为独立开源工具，**与 Apple Inc. 无任何关联、背书或赞助关系。**

---

## ⚡ 核心隔离特性

1. **🐧 Linux 深度内核隔离 (`apple::isolation::linux`)**:
   * **Linux 命名空间**: 非特权容器隔离 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
   * **cgroups v2 控制器**: 在 `/sys/fs/cgroup/apple_sandbox/{task_id}` 下精准控制内存限额 (`memory.max`)、CPU 配额 (`cpu.max`) 和核心亲和性 (`cpuset.cpus`)。
   * **seccomp-bpf 过滤**: 过滤危险系统调用（`ptrace`、离线状态下的原始套接字绑定、内核模块加载等）。

2. **🪟 Windows 安全与 Job Objects (`apple::isolation::windows` & `apple::isolation::process`)**:
   * **Job Objects**: 硬件限制 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) 以及通过 `QueryInformationJobObject` 精准统计峰值内存。
   * **受限令牌与低完整性级别**: 剥离管理员权限并降低令牌至低完整性级别 (`SECURITY_MANDATORY_LOW_RID`)。
   * **AppContainer 隔离**: 支持 Windows AppContainer 原生沙箱隔离。

3. **🍎 macOS Seatbelt 策略配置 (`apple::isolation::macos`)**:
   * **SBPL 沙箱策略语言**: 生成冻结文件系统访问与进程执行的策略 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
   * 自动通过 `sandbox-exec` 包装 `clang`、`swiftc` 和 `rustc` 等编译器命令。

4. **🔍 实时 Live I/O 与秘密探测拦截器 (`apple::isolation::interceptor` & `apple::monitor`)**:
   * 实时监测构建进程访问路径。
   * 针对机密文件（`.env`、`id_rsa`、`.aws/credentials`、`/etc/shadow`、`/root`）的探测立即产生违规警报。
   * 校验输入头文件与文件是否在 DAG 挂载规则内声明。

5. **硬链接镜像沙箱 (`apple::isolation::fs`)**:
   * 通过硬链接将源码树镜像至独立 Jail 目录，支持跨文件系统自动降级复制。

6. **11+ 语言工具链离线策略 (`apple::isolation::net`)**:
   * 注入严格的离线环境变量（Cargo、Go、pip、npm/yarn/pnpm、Maven、Gradle、.NET、Swift、Dart）。

7. **双通道确定性重现验证 (`apple::verifier`)**:
   * 在隔离环境中进行双次构建，第二次引入干扰的时间与区域变量 (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`) 并校验 BLAKE3 哈希。

---

## 🚀 CLI 命令参考

### 1. 启动 IPC 守护进程
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. 单次沙箱执行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 验证构建输出确定性
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. 查看审计记录
```bash
apple audit
```
