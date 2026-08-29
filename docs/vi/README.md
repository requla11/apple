# 🍎 Apple: Daemon Sandbox Hermetic & Cách Ly Tiến Trình cho Fish

> 🌐 **Điều hướng ngôn ngữ:**
> [English](../../README.md) | **Tiếng Việt** | [日本語](../ja/README.md) | [简体中文](../zh-hans/README.md) | [繁體中文](../zh-hant/README.md)

---

## 🎯 Tổng quan

**Apple** là một daemon sandbox hermetic ở mức tiến trình, đóng vai trò bổ trợ
cho engine điều phối build [Fish](https://github.com/requla11/fish). Trong khi
Fish lo về đồ thị phụ thuộc, cache và lập lịch song song, Apple bọc từng lệnh
build trong một môi trường kiểm soát: bộ biến môi trường đã được làm sạch, bản
làm việc trong thư mục scratch, cờ offline ở mức toolchain và timeout bắt buộc
(cùng Windows Job Object trên Windows).

Apple là một Rust library (được `fish-sandbox` sử dụng) đồng thời là CLI/daemon
độc lập.

> **Về cái tên:** "Apple" chỉ là tên dự án song sinh với Fish 🐟. Đây là công
> cụ open-source độc lập và **không liên quan, không được chứng thực hay tài
> trợ bởi Apple Inc.**

## ⚡ Apple thực sự làm gì

1. **Sandbox nhân bản hard-link (`apple::isolation::fs`)**:
   * Nhân bản cây mã nguồn vào thư mục jail theo từng task bằng hard link
     (tự động fallback sang copy khi khác filesystem).
   * Compiler ghi vào jail, cây mã nguồn gốc không bị thay đổi.

2. **Làm sạch môi trường (`apple::isolation::env`)**:
   * Loại bỏ mọi biến môi trường ngoài allow-list (cộng các tiền tố `FISH_*`
     và `APPLE_*`), trỏ `TMPDIR`/`TEMP`/`TMP` vào jail.

3. **Hạn chế mạng ở mức best-effort (`apple::isolation::net`)**:
   * Tiêm biến proxy blackhole và cờ offline được Cargo, Go, pip, npm tôn trọng
     (`CARGO_NET_OFFLINE`, `GOPROXY=off`, ...).
   * **Đây không phải firewall.** Một tiến trình bỏ qua biến proxy vẫn có mạng.
     Thực thi cứng ở mức kernel (network namespace) chưa được cài đặt.

4. **Cách ly tiến trình (`apple::isolation::process`)**:
   * **Windows**: Job Object với `KILL_ON_JOB_CLOSE` và giới hạn RAM tùy chọn;
     `CREATE_NO_WINDOW` cho tiến trình con.
   * **Unix**: cách ly nhóm tiến trình bằng `setpgid` và timeout cứng.
   * Đây là cách ly tiến trình ở user-space — không có namespace, seccomp hay
     AppContainer.

5. **Kiểm tra tất định 2 lượt build (`apple::verifier`)**:
   * Chạy cùng một lệnh build 2 lần trong jail riêng biệt; lượt 2 chạy với bộ
     biến locale/thời gian bị xáo trộn (`SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL`).
   * So sánh hash BLAKE3 của artifact. Đây là kiểm tra tự khai báo, **không
     phải** chứng nhận SLSA.

6. **Bản ghi audit (`apple::audit`)**:
   * Daemon ghi kết quả thực thi (exit code, thời lượng, vi phạm) ra JSON tại
     `<scratch>/audit/<task_id>.json` để CLI đọc lại.

7. **Kiểm tra vi phạm (`apple::monitor`)**:
   * Bộ kiểm tra chính sách theo tiền tố đường dẫn, chỉ dùng được ở dạng
     library. Chưa gắn với chặn I/O syscall thời gian thực.

## 🚀 Tham chiếu CLI

### 1. Chạy daemon IPC
```bash
apple daemon --scratch-dir .apple-scratch --socket apple.sock
```
Phục vụ JSON phân tách bằng dòng (`DaemonMessage`) qua Unix socket hoặc named
pipe trên Windows, dừng khi nhận `Shutdown` hoặc Ctrl+C.

### 2. Thực thi trong sandbox một lần
```bash
apple run --offline --memory-limit-mb 4096 --timeout-seconds 300 -- cargo build --release
```

### 3. Kiểm tra tính tất định của output
```bash
apple verify-reproducible --artifact target/release/my_bin -- cargo build --release
```
Yêu cầu build tạo artifact **bên trong** jail để cả hai lượt đều hash được.

### 4. Kiểm tra trạng thái daemon
```bash
apple status --socket apple.sock
```
Ping daemon thật qua IPC và báo khả năng kết nối, phiên bản, số sandbox đang chạy.

### 5. Xem bản ghi audit
```bash
apple audit <task_id>
apple telemetry <task_id>
```
Đọc bản ghi JSON do daemon ghi trước đó. Nếu chưa có bản ghi, CLI báo rõ —
không bao giờ in số liệu giả.

### 6. Tự nhận diện ngôn ngữ dự án
```bash
apple profile-detect --dir .
```

## 🧪 Hạn chế đã biết

* Không có sandbox mức kernel (không namespace/seccomp trên Linux, không
  AppContainer/AppLocker trên Windows).
* Chặn mạng chỉ mang tính khuyến nghị (qua biến môi trường), không thực thi cứng.
* Bộ kiểm tra vi phạm chỉ là checker đường dẫn dạng library, không phải bộ chặn
  I/O thời gian thực.
* Chưa đo CPU time và bộ nhớ đỉnh; telemetry chỉ báo những gì runner thực sự
  biết (exit code, thời lượng).
* Verifier tất định yêu cầu artifact được tạo bên trong jail.
* IPC chỉ chạy trên một máy (Unix socket / named pipe).

## 📄 Giấy phép & Miễn trừ

Phát hành theo MIT License. Xem [LICENSE](../../LICENSE) để biết chi tiết.

> **Miễn trừ:** Dự án này là công cụ open-source độc lập, không liên quan,
> không được chứng thực hay tài trợ bởi Apple Inc.
