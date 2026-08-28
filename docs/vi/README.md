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
│  (Job Objects / Namespaces)  │  (Giám sát vi phạm IO)       │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Các tính năng cốt lõi

1. **Lồng cách ly hệ thống tệp kín (`apple::isolation::fs`)**:
   * Gắn kết cây thư mục mã nguồn ở chế độ chỉ đọc (Read-Only).
   * Chuyển hướng mọi thao tác ghi tạm thời của compiler vào thư mục scratch dùng một lần, chống làm bẩn workspace.

2. **Chặn kết nối mạng Zero-Trust (`apple::isolation::net`)**:
   * Chặn đứng mọi kết nối ra Internet trong quá trình biên dịch để đảm bảo artifact tạo ra có thể tái lập 100%.

3. **Làm sạch môi trường (`apple::isolation::env`)**:
   * Loại bỏ các biến môi trường ngẫu nhiên (`USER`, `PWD`, `HOME`, `TEMP`) nhưng giữ lại các cờ biên dịch chuẩn (`RUSTFLAGS`, `CFLAGS`, `NODE_ENV`).

4. **Giám sát vi phạm thời gian thực (`apple::monitor`)**:
   * Phát hiện và ghi nhận ngay lập tức khi trình biên dịch cố tình truy cập file trái phép ra ngoài lồng cách ly.

5. **Giao thức IPC siêu tốc (`apple::protocol`)**:
   * Kết nối trực tiếp với Fish qua Unix Domain Sockets hoặc Windows Named Pipes với độ trễ dưới 1 mili-giây.

---

## 🚀 Khởi động nhanh

```bash
# Chạy daemon Apple
apple --scratch-dir .apple-scratch --socket apple.sock
```

---

## 📄 Bản quyền

Phát hành theo giấy phép MIT License. Xem [LICENSE](../../LICENSE) để biết chi tiết.
