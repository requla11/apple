# 🍎 Apple: Trình Daemon Hộp Cát Hermetic & Cô Lập Tiến Trình Cho Fish

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](../../README.md) | [Tiếng Việt](README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)
>
> 🗺️ **[Xem Toàn Bộ Lộ Trình Kỹ Thuật (ROADMAP)](ROADMAP.md)**


---

## 🎯 Giới thiệu tổng quan

**Apple** là hệ thống daemon cô lập tiến trình và hộp cát khép kín (hermetic sandbox) hiệu năng cao, bổ trợ toàn diện cho công cụ điều phối build [Fish](https://github.com/requla11/fish). Trong khi Fish điều phối đồ thị phụ thuộc DAG, bộ nhớ cache và lập lịch song song, Apple bọc từng câu lệnh build trong một môi trường được kiểm soát tuyệt đối: thư mục jail liên kết cứng (hard-link), bộ biến môi trường tinh giản, chính sách ngắt mạng 11+ ngôn ngữ, cô lập cấp độ nhân hệ điều hành (Linux Namespaces, cgroups v2, seccomp-bpf, Windows Job Objects / Restricted Tokens / AppContainer, và macOS Seatbelt SBPL), cùng bộ đón chặn vi phạm I/O thời gian thực.

Apple được phân phối dưới dạng thư viện Rust (được `fish-sandbox` / `fish-executor` sử dụng) và công cụ dòng lệnh/daemon độc lập.

> **Lưu ý về tên gọi:** "Apple" là tên dự án đồng hành cùng Fish 🐟. Dự án này là công cụ mã nguồn mở độc lập và **không liên kết, bảo trợ hoặc tài trợ bởi Apple Inc.**

---

## ⚡ Các tính năng cô lập cốt lõi

1. **🐧 Cô lập tầng sâu Kernel Linux (`apple::isolation::linux`)**:
   * **Linux Namespaces**: Cô lập container không đặc quyền (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`).
   * **Bộ điều khiển cgroups v2**: Kiểm soát chính xác hạn ngạch phần cứng tại `/sys/fs/cgroup/apple_sandbox/{task_id}` cho RAM (`memory.max`), quota CPU (`cpu.max`), và core affinity (`cpuset.cpus`).
   * **Bộ lọc seccomp-bpf**: Lọc chính sách gọi hệ thống syscall, chặn các syscall nguy hiểm (`ptrace`, bind raw socket khi offline, thao tác nạp kernel module).

2. **🪟 Bảo mật Windows & Job Objects (`apple::isolation::windows` & `apple::isolation::process`)**:
   * **Job Objects**: Giới hạn phần cứng (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) và thống kê chính xác lượng RAM đỉnh qua `QueryInformationJobObject`.
   * **Restricted Tokens & Low Integrity**: Tước bỏ quyền quản trị viên và hạ mức toàn vẹn của token xuống Low Integrity (`SECURITY_MANDATORY_LOW_RID`).
   * **Hồ sơ AppContainer**: Hỗ trợ môi trường hộp cát Windows AppContainer.

3. **🍎 Cấu hình macOS Seatbelt (`apple::isolation::macos`)**:
   * **SBPL (Sandbox Profile Language)**: Tạo hồ sơ hộp cát đóng băng quyền truy cập hệ thống tệp và thực thi tiến trình (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`).
   * Bọc lệnh thực thi thông qua `sandbox-exec` cho các trình biên dịch như `clang`, `swiftc`, và `rustc`.

4. **🔍 Đón chặn I/O thời gian thực & rò rỉ bí mật (`apple::isolation::interceptor` & `apple::monitor`)**:
   * Kiểm tra các đường dẫn truy cập theo thời gian thực.
   * Cảnh báo ngay lập tức nếu phát hiện đọc trộm các file bí mật (`.env`, `id_rsa`, `.aws/credentials`, `/etc/shadow`, `/root`).
   * Xác minh các header/file đầu vào có khớp với khai báo DAG trong mount rules hay không.

5. **Hộp cát liên kết cứng Hard-link (`apple::isolation::fs`)**:
   * Nhân bản cây mã nguồn vào thư mục jail của tác vụ bằng hard-link với cơ chế tự động fallback sao chép khi khác filesystem.

6. **Chính sách ngắt mạng 11+ ngôn ngữ (`apple::isolation::net`)**:
   * Tiêm biến môi trường offline cho Cargo, Go, pip, npm/yarn/pnpm, Maven, Gradle, .NET, Swift, Dart.

7. **Kiểm định tính tái lập Dual-Pass (`apple::verifier`)**:
   * Chạy build 2 lần trong jail mới với biến thời gian/locale bị nhiễu (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`) và đối chiếu hash BLAKE3.

---

## 🚀 Hướng dẫn CLI

### 1. Khởi động daemon IPC
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. Thực thi lệnh trong hộp cát
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Kiểm định tính tái tạo của artifact
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. Kiểm tra nhật ký kiểm toán
```bash
apple audit
```
