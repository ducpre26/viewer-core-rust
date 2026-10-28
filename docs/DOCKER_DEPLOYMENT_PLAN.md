# Plan build và triển khai Docker

Ngày chốt: 2026-10-28. Phạm vi: server thử nghiệm; không phải xác nhận production
hay hoàn tất Step 12/toàn bộ migration. Hướng dẫn sử dụng:
[Docker deployment](../deploy/docker/README.md).

## Các quyết định đã thống nhất

- Build từ Windows hoặc Linux, cùng Dockerfile, đích `linux/amd64`.
- Windows dùng Docker Desktop với Linux containers; Linux dùng Docker Engine và
  Buildx. Git và Docker là công cụ host; Rust/Java/Maven/Python build trong image.
- Rust theo rust-toolchain.toml và Cargo.lock. Temurin 25 JDK/Ubuntu 24.04 dùng
  cùng digest trong builder/runtime; giữ Java facade/JavaCV/FFmpeg hiện tại.
- Deploy một container app bằng Compose, dùng PostgreSQL riêng của Rust đã có
  và DICOM Store bên ngoài. Chuyển image bằng save/load; không cần registry.
- Java, facade và native directory có đường dẫn cố định trong image. Người vận
  hành cấu hình volume, không cài hoặc tìm đường dẫn Java trên server.
- Bỏ yêu cầu `10001:10001` theo xác nhận cuối: không đặt USER trong Dockerfile
  hoặc user trong Compose; kế thừa user mặc định của image nền.
- Không sửa cấu hình local đang có; cấu hình Docker mẫu ở thư mục riêng.

Máy chạy lệnh build và nền tảng image là hai việc riêng: Windows hoặc Linux
đều tạo image `linux/amd64`, deploy lên server Linux x86_64 có Docker. Image
chứa binary/thư viện Linux cho CPU Intel/AMD 64-bit, không phải image chạy trên
mọi OS/CPU. Trên Windows, Docker Desktop dùng môi trường Linux qua WSL 2;
người dùng vẫn chạy script từ PowerShell.

## Phân biệt cấu hình build và cấu hình chạy

| Thời điểm | Cấu hình |
|---|---|
| Build image | Platform linux/amd64, tag image, Rust/JDK/Maven, dependencies, Git revision/dirty |
| Chạy container | Image tag, cổng publish, CPU/RAM, volume và restart policy trong Compose |
| Khởi động ứng dụng | DB/auth/session/logging, JNI memory/timeout và giới hạn download trong TOML |

Không đưa credential hoặc địa chỉ DB/DICOM Store thật vào image/build args.
Đổi source/dependency cần build image mới. Đổi TOML thì restart app; đổi
Compose thì recreate bằng compose up. Ứng dụng chưa tự reload TOML.

## Build và nguồn gốc artifact

Hai script PowerShell/Bash lấy Git revision và dirty state, truyền build args
cho Docker. Công cụ facade nhận hai tham số metadata đi cùng nhau để build
không có `.git`; local build cũ vẫn có thể đọc Git. Manifest và image label
ghi metadata đó. Dirty=true chỉ nhận diện có thay đổi, không lưu lại diff.

Build context dùng allowlist, loại credential, config local, docs/STORE.md và
artifact máy host. Các stage build tạo binary Linux và distribution facade;
runtime chỉ copy kết quả, không copy Cargo/Maven/Python. Facade giữ nguyên
inventory/checksum; không loại native JAR sau khi tạo manifest.

Base Temurin được tra Docker Hub và ghim digest; apt packages và dependency
downloads chưa snapshot toàn bộ, nên không tuyên bố build byte-for-byte tái lập.

## Layout và cấu hình chạy

| Đường dẫn container | Nguồn / quyền |
|---|---|
| `/opt/java/openjdk` | JDK trong image; java_home cố định |
| `/app/facade` | JAR, dependencies, manifest trong image |
| `/app/native` | Thư mục có sẵn trong image cho java.library.path |
| `/app/config` | Bind mount config/app.toml + app-prod.toml, chỉ đọc |
| `/var/lib/viewer/jni` | Volume ghi được; JavaCPP cache tại jni/javacpp |
| `/var/lib/viewer/download` | Cùng volume, file download tạm |

Không mount đè Java/facade/native. Không khai báo libjvm_path khi đã dùng
java_home. Entrypoint tạo thư mục con trên volume, kiểm tra ghi được và exec
binary; Summer tiếp tục nạp TOML/profile như hiện tại. Không thêm config loader.

Compose mặc định bind host 127.0.0.1:8888, cho phép đổi địa chỉ/cổng publish;
app luôn nghe 0.0.0.0:8888 trong container. SUMMER_ENV=prod, credential ở file
TOML server, restart unless-stopped, JSON log xoay vòng 10 MiB x 3.

Thông số thử nghiệm: 2 CPU/4 GiB container; JVM Xms=128 MiB, Xmx=512 MiB,
direct=128 MiB, check_jni=true. Một download; input 128 MiB, output 256 MiB,
job 1 GiB, metadata 8 MiB, 1.000 entry, 512 frame, 100 triệu pixel, 10 FPS.
Download deadline 120 giây; JNI startup/probe 120 giây; shutdown JVM 30 giây;
Docker stop grace 180 giây. Cleanup TTL 1 giờ, interval 60 giây.

Xmx/direct không giới hạn toàn bộ native FFmpeg/Rust memory. Job limit không
phải quota vật lý ổ đĩa. JNI timeout không hủy native encode; deadline download
có thể hết trong khi worker vẫn giữ file/permit tới completion. HTTP drain
có thể vượt stop grace khi client chậm. Thông số này chưa chốt P1–P4 production.

## Kiểm chứng và bàn giao

Kiểm tra script Windows/Linux, build không có .git, đúng platform, manifest và
checksum. Chạy đúng image với PostgreSQL test: startup/migration/readiness,
session/WADO, ZIP và MP4 thật, cache rỗng/có sẵn, quyền volume, lỗi DB/config,
SIGTERM khi encode/stream, restart và cleanup. Đo RSS của workload synthetic;
không suy rộng thành production sizing. Test bỏ qua không tính đạt.

Healthcheck gọi /ws/rest/ready; unhealthy không tự restart container còn sống.
Restart policy xử lý process thoát. Không xóa volume khi cập nhật/rollback.
Rollback image không hoàn tác migration; restart làm mất session chỉ có cache.

Tiến độ đóng gói và kết quả kiểm tra thực tế ghi tại
[Step 18](step/step-18-hardening.md). Docker runtime chưa được kiểm chứng trên
máy hiện tại vì chưa có Docker CLI/engine. Không commit/push/deploy tự động.
