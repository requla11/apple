# 🍎 Apple: Daemon Cách ly Tiến trình & Sandbox Kín Cấp Kernel

> 🌐 **Chuyển đổi ngôn ngữ:**
> [English](../../README.md) | [Tiếng Việt](README.md) | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 Tổng quan

**Apple** là daemon cách ly tiến trình và sandbox kín (hermetic) cấp kernel được thiết kế chuyên biệt để bổ trợ cho hệ thống điều phối biên dịch đa ngôn ngữ [Fish](https://github.com/requla11/fish).

Trong khi Fish đóng vai trò là bộ não điều phối đồ thị phụ thuộc (DAG), bộ nhớ đệm CAS và thuật toán biên dịch đón đầu, thì **Apple** hoạt động như tấm khiên bảo vệ cấp hệ thống, đảm bảo mọi lệnh biên dịch (`rustc`, `gcc`, `go`, `tsc`) đều được thực thi trong một môi trường cách ly tuyệt đối, không rò rỉ file rác và tái lập được 100%.

```text
┌─────────────────────────────────────────────────────────────┐
│                 🐟 Fish Build Orchestrator                  │
└──────────────────────────────┬──────────────────────────────┘
                               │ IPC (Unix Domain Socket / Named Pipe)
┌──────────────────────────────▼──────────────────────────────┐
│                   🍎 Apple Sandbox Daemon                   │
├──────────────────────────────┬──────────────────────────────┤
│  Hermetic Filesystem Manager │  Network Lockdown Controller │
│  (Lồng cách ly & Overlay)    │  (Chặn mạng tuyệt đối)       │
├──────────────────────────────┼──────────────────────────────┤
│  Process Isolation Runner    │  Real-Time Violation Monitor │
│  (Job Objects & Namespaces)  │  (Giám sát vi phạm IO)       │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Các tính năng cốt lõi

1. **Lồng cách ly hệ thống tệp kín (`apple::isolation::fs`)**:
   * Gắn kết cây thư mục mã nguồn ở chế độ chỉ đọc (Read-Only).
   * Chuyển hướng mọi thao tác ghi tạm thời của compiler vào thư mục scratch dùng một lần.

2. **Chặn kết nối mạng Zero-Trust (`apple::isolation::net`)**:
   * Chặn đứng mọi kết nối ra Internet trong quá trình biên dịch để đảm bảo artifact tạo ra có thể tái lập 100%.

3. **Cách ly tiến trình OS-Native cấp Kernel (`apple::isolation::process`)**:
   * **Windows**: Sử dụng Windows Native Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, chặn rò rỉ RAM).
   * **Unix / Linux**: Sử dụng nhóm tiến trình `setpgid` và cơ chế kiểm soát timeout nghiêm ngặt.

4. **Làm sạch môi trường (`apple::isolation::env`)**:
   * Loại bỏ các biến môi trường ngẫu nhiên (`USER`, `PWD`, `HOME`, `TEMP`) nhưng giữ lại các cờ biên dịch chuẩn (`RUSTFLAGS`, `CFLAGS`, `NODE_ENV`).

5. **Giám sát vi phạm & Kho Audit Report (`apple::monitor`, `apple::audit`)**:
   * Ghi nhận và truy xuất báo cáo về mọi hành vi truy cập tệp tin trái phép của compiler.

---

## 🚀 Hướng dẫn sử dụng CLI

### 1. Thực thi một lệnh trực tiếp trong Sandbox
```bash
# Chạy lệnh trong sandbox offline kín, giới hạn 4GB RAM và 300s timeout
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 2. Chạy dưới dạng Daemon nền
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```

### 3. Kiểm tra trạng thái Daemon
```bash
apple status --socket apple.sock
```

### 4. Xem báo cáo Audit vi phạm
```bash
apple audit build_target_01
```

---

## 📄 Bản quyền

Phát hành theo giấy phép MIT License. Xem [LICENSE](../../LICENSE) để biết chi tiết.
