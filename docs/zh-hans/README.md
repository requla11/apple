# 🍎 Apple: Fish 构建系统的密封沙箱与进程隔离守护程序

> 🌐 **多语言导航 / Language Navigation:**
> [English](../../README.md) | [Tiếng Việt](../vi/README.md) | [日本語](../ja/README.md) | [简体中文](README.md) | [繁體中文](../zh-hant/README.md)
>
> 🗺️ **[查看完整技术路线图 (Roadmap)](ROADMAP.md)**

---

## 🎯 概述

**Apple** 是为 [Fish](https://github.com/requla11/fish) 构建编排系统及企业级独立工具链设计的高性能进程级密封沙箱引擎与隔离守护程序。在 Fish 负责 DAG 依赖图与分布式缓存的同时，Apple 将编译器及工具链指令包装在极具安全保障的受控环境中：内核级沙箱、写时复制 (CoW) 零拷贝存储隔离、实时分块流式 IPC、原子级任务取消以及 SLSA v1.0 / SPDX / CycloneDX 供应链安全。

> **名称说明：** "Apple" 是 Fish 🐟 的协同项目代号。本项目是一个独立的开源工具，**与 Apple Inc. 没有任何关联、认可或赞助关系。**

---

## ⚡ 核心架构特性

### 1. 🐧 Linux 深度内核隔离 (`apple::isolation::linux`)
- **Linux 命名空间**: 无特权容器级隔离 (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`)。
- **cgroups v2 控制器**: 在 `/sys/fs/cgroup/apple_sandbox/{task_id}` 限制硬件配额：内存 (`memory.max`)、CPU 配额 (`cpu.max`) 及核心亲和度 (`cpuset.cpus`)。
- **seccomp-bpf 过滤**: 系统调用级安全策略，阻断危险调用 (`ptrace`、离线时创建裸套接字、加载内核模块)。
- **Landlock LSM**: Linux 内核级路径访问控制规则，实施细粒度读写管控。

### 2. 🪟 Windows 安全与作业对象 (`apple::isolation::windows`)
- **Job Objects**: 硬件限制 (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) 及精准峰值内存统计。
- **Restricted Tokens 与低完整性**: 剥离管理员权限并将令牌完整性降为 `SECURITY_MANDATORY_LOW_RID`。
- **AppContainer 配置文件**: 原生 Windows AppContainer 隔离支持。

### 3. 🍏 macOS Seatbelt 配置文件 (`apple::isolation::macos`)
- **沙箱配置语言 (SBPL)**: 动态生成密封配置 (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`）。
- 使用 `sandbox-exec` 直接包装原生编译器 (`clang`, `swiftc`, `rustc`)。

### 4. ⚡ 零拷贝存储隔离 (`apple::isolation::cow` & `fs`)
- **写时复制块克隆 (CoW Block Cloning)**: 硬件加速支持 APFS `clonefile(2)`、Linux `FICLONE` / `Btrfs` reflink 及 Windows FSCTL 块克隆（附带硬链接回退）。
- **差异构件同步 (Differential Sync)**: 自动对比元数据快照以提取变动和新生成的构建产物。

### 5. 🌊 实时流式 IPC 与进程取消 (`apple::protocol` & `daemon`)
- **分块流式传输**: 通过 Unix Domain Sockets / Windows Named Pipes 异步非阻塞传输 stdout/stderr 数据块 (4KB 缓冲区)。
- **进程组终结**: 通过 Unix `SIGKILL` 进程组及 Windows Job Object 关闭实现原子级即时取消。

### 6. 🔐 企业级供应链安全与 SLSA v1.0 (`apple::provenance`, `attestation`, `sbom`)
- **SLSA v1.0 来源溯源**: 生成符合 in-toto Statement v1 标准的构建来源元数据，内含 BLAKE3 哈希。
- **密码学证明签署 (Attestation)**: 使用带密钥的 BLAKE3 MAC 签署并验证证明信封。
- **自动 SBOM 生成**: 导出国际标准 **SPDX 2.3** 与 **CycloneDX 1.5** 格式的软件物料清单。

---

## 🚀 CLI 命令行使用说明

### 1. 启动 IPC 守护程序
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. 单次沙箱执行
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. 双遍可重现构建验证
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. 生成 SLSA v1.0 来源元数据
```bash
apple provenance --task-id task_123 --artifacts target/release/my_bin --output provenance.json
```

### 5. 导出软件物料清单 (SPDX 2.3 / CycloneDX 1.5)
```bash
apple sbom --format spdx --task-id task_123 --artifacts target/release/my_bin --output sbom.spdx.json
apple sbom --format cyclonedx --task-id task_123 --artifacts target/release/my_bin --output sbom.cdx.json
```

### 6. 签署与验证 Attestation 信封
```bash
# 签署
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# 验证
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --verify --envelope envelope.json
```
