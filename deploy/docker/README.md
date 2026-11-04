# Build Windows/Linux, deploy Docker Linux amd64

Bộ này dùng cho **server thử nghiệm**, một app instance, PostgreSQL và DICOM
Store bên ngoài. Java 25/facade/FFmpeg nằm trong image. Không cần Java trên host.
Plan: [DOCKER_DEPLOYMENT_PLAN](../../docs/DOCKER_DEPLOYMENT_PLAN.md).

Luồng bàn giao: **build trên Windows/Linux → image linux/amd64 → export .tar
→ load trên server Linux x86_64 → mount config/data → chạy Compose**.

Image có nền tảng OS/CPU cụ thể. Windows là máy chạy lệnh build; Docker Desktop
dùng Linux qua WSL 2 để tạo image Linux, không tạo binary Windows cho container.
Server deploy cần Linux x86_64 (Intel/AMD 64-bit); ARM64/Windows containers
không thuộc bộ triển khai này.

| Cấu hình lúc build | Cấu hình lúc chạy |
|---|---|
| Platform, image tag, phiên bản toolchain/dependency, Git metadata | Compose: image, cổng, volume, CPU/RAM, restart |
| Không đưa credential server vào image | TOML: DB/auth/session, JNI memory/timeout, video.fps, logger |

Đổi source/dependency thì build image mới. Đổi cấu hình ứng dụng thì sửa TOML
và restart; đổi Compose thì dùng compose up để áp dụng. Java/facade nằm sẵn
trong image, người triển khai không cần chọn đường dẫn Java trên host.

## 1. Chuẩn bị máy build

- Git và checkout repo.
- Windows: Docker Desktop đang chạy, chọn Linux containers (WSL 2).
- Linux: Docker Engine và Buildx.
- Docker Compose plugin để chạy thử/deploy.
- Internet tới Docker Hub, Ubuntu apt, Rust registry/static.rust-lang.org,
  Maven Central, archive.apache.org và maven.dcm4che.org.

Rust/Java/Maven/Python được chuẩn bị trong build stages. Host không cần cài
chúng. Image đích luôn linux/amd64, kể cả chạy lệnh từ Windows.

Từ root repo, PowerShell:

```powershell
.\scripts\build-docker.ps1 -Image viewer-core-rust:test-001 -ExportPath "$env:TEMP\viewer-core-rust-test-001.tar"
```

Linux:

```bash
bash scripts/build-docker.sh --image viewer-core-rust:test-001 --export /tmp/viewer-core-rust-test-001.tar
```

Bỏ tham số export nếu chỉ muốn build và load image vào Docker local. Thư mục
cha của file export phải tồn tại; script từ chối ghi đè file đã có. Không tự push.
Script in Git revision/dirty; nếu dirty=true, commit hash không đại diện đầy đủ
các thay đổi đang build. Chọn tag mới cho mỗi bản để quay lại bản cũ được.

Build stages cài Maven 3.9.11 và Rust theo rust-toolchain.toml; runtime-base
ghim Temurin 25 JDK Noble bằng digest trong Dockerfile. Cập nhật digest cần
chạy lại kiểm tra Linux/JNI/codec. Build facade verify chỉ xác minh artifact,
không thay bài kiểm tra MP4 thật trên runtime image.

## 2. Chuẩn bị server

Copy file image .tar và toàn bộ thư mục deploy/docker vào server, ví dụ
`/srv/viewer-core-rust`. Server cần Docker Engine/Compose, không cần source,
compiler hoặc Maven.

```bash
cd /srv/viewer-core-rust
docker load --input /path/to/viewer-core-rust-test-001.tar
# Chỉ làm một lần; giữ nguyên config thật khi nâng cấp.
test ! -e config && cp -R config.example config
```

Sửa `config/app-prod.toml`: DB host/database/user/password, admin username/password.
Thay tất cả giá trị có tiền tố REPLACE_WITH_ trước khi chạy. Entrypoint từ chối
mẫu chưa điền. Dùng PostgreSQL test riêng của Rust; app tự chạy migration và
cần quyền DDL. Không trỏ vào DB production để thử image lần đầu.

DB URL không chứa username/password. Host phải truy cập được từ container;
`localhost` là container, không phải server. Nếu DB ở cùng máy Linux host, dùng
địa chỉ host truy cập được và cấu hình PostgreSQL/firewall cho mạng Docker.

`app.toml` và `app-prod.toml` cùng được mount chỉ đọc. Không sửa config trong
image, không copy secret vào Dockerfile/build args. File config thật trong repo
được gitignore. Có thể đặt public_domain/path_prefix trong overlay theo frontend.

## 3. Volume và JNI

Compose dùng named volume `viewer-data` tại `/var/lib/viewer`. Entrypoint tạo
`jni/` và `pacs-viewer-core-downloads/`, kiểm tra quyền ghi, rồi exec app. Image đặt
`TMPDIR=/var/lib/viewer` để vùng download tạm nằm trên volume. Không đặt user cố định;
kế thừa user image nền. Không cần tạo UID/GID 10001 trên host.

Muốn lưu ở đường dẫn host cụ thể, thay dòng volume dữ liệu bằng:

```yaml
      - /srv/viewer-data:/var/lib/viewer
```

Tạo `/srv/viewer-data` trước và bảo đảm container ghi được. Host có SELinux
cần cấu hình nhãn phù hợp. Volume JavaCPP phải cho phép nạp native library
được giải nén; không đặt cache trên filesystem ngăn executable mappings.

Không mount Java hoặc facade từ host. Các đường dẫn JNI cố định trong config
mẫu khớp image: Java `/opt/java/openjdk`, facade `/app/facade`, native directory
`/app/native`. JavaCPP giải nén ở `/var/lib/viewer/jni/javacpp`, không phải
`/app/native`. Native directory được tạo sẵn, không cần volume riêng.

## 4. Chạy và kiểm tra

```bash
export VIEWER_IMAGE=viewer-core-rust:test-001
docker compose config --quiet
docker compose up -d --wait --wait-timeout 180
docker compose ps
docker compose logs --tail 100 viewer
curl --fail http://127.0.0.1:8888/ws/rest/health
curl --fail http://127.0.0.1:8888/ws/rest/ready
```

Compose chọn image có sẵn (`pull_policy: never`); không build hoặc pull trên
server. Mặc định chỉ host truy cập cổng publish. Để mở trên mạng server:

```bash
export VIEWER_LISTEN_ADDRESS=0.0.0.0
export VIEWER_HOST_PORT=8888
docker compose up -d --wait --wait-timeout 180
```

Có thể lưu ba biến VIEWER_IMAGE/VIEWER_LISTEN_ADDRESS/VIEWER_HOST_PORT trong
`.env` cạnh Compose để dùng lại. Chúng điều khiển Docker, không thay TOML loader.
DB/auth/JNI dùng TOML. Cổng nội bộ vẫn 8888 để healthcheck hoạt động.

Mẫu thử nghiệm bật JNI, download luôn hoạt động, `check_jni=true`, container 2 CPU/4 GiB RAM.
Xmx và direct memory không bao gồm toàn bộ Rust/JVM/FFmpeg native memory.
`jni.timeout_ms` dùng chung cho startup/control probe và từng lời gọi JNI.
Không còn section `[download]` hoặc deadline toàn request; `[video].fps` mặc định 1.
Hết deadline JNI không buộc dừng FFmpeg đang chạy. Readiness
có thể báo 503 khi worker quá deadline rồi hồi phục khi encode kết thúc.

Healthcheck không tự restart container còn sống. `unless-stopped` xử lý process
thoát. Healthcheck chờ tối đa 5 giây mỗi lần, có thể hết trước timeout probe
120 giây; kiểm tra cả logs khi unhealthy. Server restart giữ named volume.

## 5. Cập nhật, dừng và rollback

Thay TOML rồi `docker compose restart viewer` để app đọc lại. Nếu sửa Compose
(image, port, volume, giới hạn), dùng `docker compose up -d --wait --wait-timeout 180`.

```bash
docker load --input /path/to/new-image.tar
export VIEWER_IMAGE=viewer-core-rust:test-002
docker compose up -d --wait --wait-timeout 180
# Quay lại tag trước khi cần; phải còn image và schema tương thích.
export VIEWER_IMAGE=viewer-core-rust:test-001
docker compose up -d --wait --wait-timeout 180
# Dừng ứng dụng, giữ dữ liệu volume.
docker compose stop
```

Không dùng `down -v` nếu cần giữ volume. Rollback image không hoàn tác migration;
cần backup DB và kiểm tra schema tương thích trước khi nâng cấp. Session chỉ ở
cache mất sau restart. Docker chờ 180 giây rồi có thể SIGKILL; HTTP stream chậm
có thể khiến shutdown vượt thời gian dù JVM shutdown_ms chỉ 30 giây.

## Giới hạn kiểm chứng

Máy phát triển hiện chưa có Docker CLI/engine. Cú pháp và hành vi script có thể
kiểm tra cục bộ, nhưng chưa tính image build hoặc runtime Linux là đạt. Phải
kiểm tra cold/warm JavaCPP cache, MP4/ZIP thật, DB/session/WADO, SIGTERM khi đang
encode/stream, restart/cleanup và tổng RSS trên image trước khi dùng rộng hơn.

Các lệnh kiểm tra hiện có ở [DEVELOPMENT_COMMANDS](../../docs/DEVELOPMENT_COMMANDS.md).
Không coi test DB/JNI bị bỏ qua là pass, và không dùng bộ Docker này để đánh dấu
toàn bộ Step 18 hoặc các phần Step 12 chưa kiểm chứng là hoàn tất.
