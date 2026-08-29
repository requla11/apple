# 🍎 Apple: Sandbox Hermetic & Daemon Cách Ly Tiến Trình Cho Fish

> 🌐 **Điều Hướng Ngôn Ngữ / Language Navigation:**
> [English](../../README.md) | [Tiếng Việt](README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)
>
> 🗺️ **[Xem Lộ Trình Kỹ Thuật Chi Tiết (Roadmap)](ROADMAP.md)**

---

## 🎯 Tổng Quan

**Apple** là engine sandbox hermetic hiệu năng cao và daemon cách ly tiến trình được thiết kế đồng hành cùng hệ thống điều phối build [Fish](https://github.com/requla11/fish) cũng như chạy độc lập cho các chuỗi công cụ enterprise. Trong khi Fish điều phối đồ thị phụ thuộc DAG và bộ nhớ cache phân tán, Apple bọc các lệnh biên dịch trong một môi trường được kiểm soát tuyệt đối: cách ly tầng sâu kernel, lưu trữ Copy-on-Write (CoW) zero-copy, streaming IPC thời gian thực, hủy tác vụ tức thì và bảo mật chuỗi cung ứng chuẩn SLSA v1.0 / SPDX / CycloneDX.

> **Ghi chú về tên gọi:** "Apple" là tên dự án đồng hành cùng Fish 🐟. Dự án này là một công cụ mã nguồn mở độc lập và **không liên kết, không được chứng thực hoặc tài trợ bởi Apple Inc.**

---

## ⚡ Các Khả Năng Kiến Trúc Cốt Lõi

### 1. 🐧 Cách Ly Tầng Sâu Linux Kernel (`apple::isolation::linux`)
- **Linux Namespaces**: Cách ly container không cần quyền root (`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, `CLONE_NEWUSER`).
- **Bộ điều khiển cgroups v2**: Giới hạn hạn ngạch phần cứng tại `/sys/fs/cgroup/apple_sandbox/{task_id}` cho RAM (`memory.max`), CPU quota (`cpu.max`), và gán lõi CPU (`cpuset.cpus`).
- **Bộ lọc seccomp-bpf**: Lọc và chặn các lệnh gọi hệ thống nguy hiểm (`ptrace`, mở socket mạng khi offline, nạp kernel module).
- **Landlock LSM**: Thiết lập các quy tắc giới hạn đường dẫn trực tiếp từ nhân Linux cho quyền đọc/ghi chi tiết.

### 2. 🪟 Bảo Mật Windows & Job Objects (`apple::isolation::windows`)
- **Job Objects**: Giới hạn phần cứng (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `KILL_ON_JOB_CLOSE`) và hạch toán đỉnh RAM chính xác qua `QueryInformationJobObject`.
- **Restricted Tokens & Low Integrity**: Tước bỏ quyền quản trị viên và hạ cấp độ tin cậy xuống `SECURITY_MANDATORY_LOW_RID`.
- **Hồ sơ AppContainer**: Hỗ trợ sandbox gốc qua Windows AppContainer.

### 3. 🍏 Hồ Sơ macOS Seatbelt (`apple::isolation::macos`)
- **Sandbox Profile Language (SBPL)**: Tự động sinh hồ sơ sandbox hermetic (`(version 1)`, `(deny default)`, `(allow process-exec ...)`, `(allow file-read* ...)`).
- Bọc trực tiếp bằng `sandbox-exec` cho các trình biên dịch gốc (`clang`, `swiftc`, `rustc`).

### 4. ⚡ Jail Lưu Trữ Zero-Copy (`apple::isolation::cow` & `fs`)
- **Copy-on-Write Block Cloning**: Tăng tốc phần cứng qua APFS `clonefile(2)`, Linux `FICLONE` / `Btrfs` reflink, và Windows FSCTL block cloning kết hợp hardlink.
- **Đồng Bộ Artifact Sai Khác (Differential Sync)**: Tự động so sánh snapshot metadata để trích xuất các artifact mới sinh ra hoặc bị chỉnh sửa.

### 5. 🌊 Streaming IPC & Hủy Tác Vụ Thời Gian Thực (`apple::protocol` & `daemon`)
- **Stream Dạng Chunks**: Đọc và truyền luồng stdout/stderr (buffer 4KB) bất đồng bộ không chặn qua Unix Domain Sockets / Windows Named Pipes.
- **Hủy Tiến Trình Nhánh**: Hủy tức thì toàn bộ nhóm tiến trình bằng Unix `SIGKILL` process groups và đóng Windows Job Object.

### 6. 🔐 Bảo Mật Chuỗi Cung Ứng & SLSA v1.0 (`apple::provenance`, `attestation`, `sbom`)
- **SLSA v1.0 Provenance**: Xuất metadata chứng minh nguồn gốc theo chuẩn in-toto Statement v1 với mã băm BLAKE3.
- **Ký Số Mật Mã (Attestation)**: Ký và xác thực phong bì chứng thực bằng mã MAC BLAKE3 có khóa bảo mật.
- **Tạo SBOM Tự Động**: Xuất danh mục thành phần phần mềm chuẩn quốc tế **SPDX 2.3** và **CycloneDX 1.5**.

---

## 🚀 Hướng Dẫn Sử Dụng CLI

### 1. Khởi chạy Daemon IPC
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 2. Thực thi Lệnh trong Sandbox Đơn Lẻ
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Kiểm Tra Tính Tái Lập (Reproducible Build)
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```

### 4. Xuất Báo Cáo Nguồn Gốc SLSA v1.0
```bash
apple provenance --task-id task_123 --artifacts target/release/my_bin --output provenance.json
```

### 5. Xuất Danh Mục SBOM (SPDX 2.3 / CycloneDX 1.5)
```bash
apple sbom --format spdx --task-id task_123 --artifacts target/release/my_bin --output sbom.spdx.json
apple sbom --format cyclonedx --task-id task_123 --artifacts target/release/my_bin --output sbom.cdx.json
```

### 6. Ký Số và Xác Thực Attestation
```bash
# Ký số
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# Xác thực
apple attest --provenance provenance.json --secret-key 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --verify --envelope envelope.json
```
