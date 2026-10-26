# Hướng dẫn cấu trúc repo Rust cho người mới

Tài liệu mô tả repo `1pacs-viewer-core-rust` đến Step 4 (2026-10-26).
Ứng dụng có HTTP, PostgreSQL, nền tảng JVM/JNI và middleware auth/session.
Đã có CRUD client/quyền và gán/bỏ gán quyền cho admin (UC-009).
Chưa triển khai thao tác DICOM nghiệp vụ.

## 1. Một số khái niệm trước khi đọc

| Khái niệm | Hiểu trong repo này |
|---|---|
| Rust | Ngôn ngữ viết backend mới; mã nguồn có đuôi `.rs`. |
| Cargo | Công cụ quản lý thư viện, build và chạy test; gần với vai trò Maven trong Java. |
| Package | Đơn vị dự án được mô tả bởi một `Cargo.toml`. Workspace hiện có `viewer-app` và `dicom-jvm`. |
| Crate | Đơn vị mã được Rust biên dịch. Package này có crate thư viện từ `lib.rs` và crate chương trình từ `main.rs`. |
| Workspace | Nhóm package cùng quản lý dependency và kết quả build. Hiện workspace có hai package. |
| Module | Cách tổ chức mã bên trong crate, khai báo bằng `mod`. Folder không tự trở thành module chỉ vì tồn tại. |
| Dependency | Thư viện bên ngoài được khai báo trong `Cargo.toml`. |
| Migration | Thay đổi cấu trúc DB được quản lý theo thứ tự để áp dụng khi nâng cấp ứng dụng. |
| Pool | Nhóm kết nối DB dùng lại giữa các request, tránh mở kết nối mới cho từng thao tác. |

## 2. Bản đồ thư mục

```text
1pacs-viewer-core-rust/
├── Cargo.toml                 Cấu hình workspace, dependency dùng chung
├── Cargo.lock                 Phiên bản dependency thực tế được khóa
├── rust-toolchain.toml        Bộ công cụ Rust được chọn cho dự án
├── README.md                  Hướng dẫn chạy và tổng quan
├── .gitignore                 Những tệp Git bỏ qua
├── .git/                      Dữ liệu quản lý phiên bản của Git
├── .github/workflows/
│   └── rust.yml               Quy trình kiểm tra tự động trên GitHub
├── config/
│   ├── app.toml               Cấu hình nền
│   └── app-prod.toml          Cấu hình bổ sung cho production
├── crates/viewer-app/
│   ├── Cargo.toml             Cấu hình package ứng dụng
│   └── src/
│       ├── main.rs            Điểm khởi động chương trình
│       ├── lib.rs             Khai báo các module dùng chung
│       ├── api/mod.rs         Route HTTP, health/readiness, kiểm thử tích hợp
│       ├── api/admin.rs       UC-009: CRUD client/quyền, mapping và guard admin
│       ├── api/admin/tests.rs Contract HTTP và hậu điều kiện PostgreSQL cho UC-009
│       ├── persistence/mod.rs Pool PostgreSQL và migration runner
│       ├── auth/mod.rs        Middleware, session cache và guard; config/test ở cùng thư mục
│       ├── domain/mod.rs      Principal, role và quy tắc permission
│       └── dicom/mod.rs       Ranh giới gọi adapter JVM từ ứng dụng
├── crates/dicom-jvm/          Adapter JVM/JNI và kiểm thử subprocess
├── java/dicom-facade/         Maven module facade Java 25
├── docs/                      Tài liệu kiến trúc, kế hoạch, tiến độ
├── tools/                     Công cụ hỗ trợ phát triển bằng Python
└── target/                    Kết quả build tự sinh
```

## 3. Các tệp ở gốc repo

### `Cargo.toml`: khai báo dự án

Tệp ở gốc khai báo `[workspace]`, trong đó `members` liệt kê package tham gia.
`[workspace.package]` chứa thông tin chung như version và Rust edition.
`[workspace.dependencies]` khai báo thư viện để các package tái sử dụng.

Khai báo ở workspace chưa có nghĩa mọi package tự động dùng thư viện đó.
Package `viewer-app` chọn dùng bằng dòng như `serde.workspace = true` trong
`crates/viewer-app/Cargo.toml`.

| Dependency hiện có | Tác dụng |
|---|---|
| `summer` | Khởi tạo và quản lý vòng đời ứng dụng, cấu hình, logging, plugin. |
| `summer-web` | HTTP dựa trên Axum, khai báo route và OpenAPI. |
| `summer-job` | Nền tảng scheduler; hiện chưa đăng ký job nghiệp vụ. |
| `tokio` | Runtime cho các tác vụ bất đồng bộ, như chờ HTTP hoặc DB. |
| `serde` | Chuyển dữ liệu Rust sang dạng tuần tự hóa; API hiện dùng để tạo JSON. |
| `schemars` | Sinh mô tả schema dữ liệu dùng cho OpenAPI. |
| `sea-orm` | Làm việc với PostgreSQL và connection pool. |
| `sea-orm-migration` | Chạy và theo dõi migration DB. |
| `url` | Phân tích URL kết nối và xử lý thành phần credential. |
| `jni` | Nạp JVM và gọi facade Java trong worker riêng. |
| `serde_json`, `sha2` | Kiểm tra capabilities JSON và checksum JAR. |

Trong khai báo dependency, `features` bật các khả năng cần dùng của thư viện.
Ví dụ SeaORM đang bật PostgreSQL và Tokio/Rustls. Phiên bản khai báo có thể là
một khoảng cho phép; phiên bản thực tế nằm trong `Cargo.lock`.

### `Cargo.lock`: khóa phiên bản đã chọn

Cargo tự tạo/cập nhật tệp này khi giải quyết dependency. Nó ghi cả thư viện trực
tiếp lẫn thư viện phụ thuộc gián tiếp. Commit tệp này giúp các máy build cùng
bộ phiên bản. Không cần đọc hết hoặc chỉnh bằng tay khi mới học.

### `rust-toolchain.toml`: chọn bộ công cụ

Dự án đang chọn Rust `1.99.0`, kèm `rustfmt` để định dạng mã và `clippy` để
phát hiện vấn đề trong code. `edition = "2024"` trong Cargo.toml là phiên bản
quy tắc ngôn ngữ; nó khác với số phiên bản compiler `1.99.0`.

### `README.md`, `.gitignore`, `.git/`

- `README.md`: nơi đọc cách cấu hình, chạy, kiểm thử và các quy ước persistence.
- `.gitignore`: bỏ qua `target/`, `.env`, log và cache Python. Repo không tự
  đọc `.env`; hiện ứng dụng đọc biến môi trường của tiến trình.
- `.git/`: lịch sử commit và dữ liệu nội bộ Git; không chỉnh nội dung bằng tay.

## 4. Đọc mã ứng dụng theo thứ tự nào?

### `src/main.rs`: ứng dụng bắt đầu từ đây

Luồng hiện tại:

```text
Chạy viewer-app
    → Đọc biến môi trường DB, tạo pool
    → Kiểm tra SELECT 1
    → Chạy các migration còn thiếu
    → Nếu JNI bật: kiểm tra runtime/artifact, tạo JVM và kiểm tra facade
    → Nạp cấu hình ứng dụng, đăng ký WebPlugin và JobPlugin
    → Đăng ký route HTTP, chạy server
    → Khi dừng: chờ server/scheduler, dừng JVM rồi đóng pool DB
```

DB bắt buộc. Nếu cấu hình hoặc kết nối/migration lỗi, chương trình kết thúc với
mã lỗi trước khi mở HTTP. `VIEWER_DATABASE_URL`, `VIEWER_DATABASE_USER` và
`VIEWER_DATABASE_PASSWORD` được đọc từ môi trường; không ghi mật khẩu vào mã.
Khi `VIEWER_JNI_ENABLED=true`, startup cũng yêu cầu runtime Java 25 và facade hợp lệ;
xem [hướng dẫn adapter](../crates/dicom-jvm/README.md).

`#[tokio::main]` chuẩn bị runtime để hàm `async fn main()` hoạt động.
`.await` chờ kết quả thao tác bất đồng bộ. `use ...` đưa tên kiểu/hàm vào phạm
vi hiện tại, tương tự mục đích của `import` trong Java.

### `src/lib.rs`: danh sách module công khai

Tệp này khai báo `pub mod api;`, `pub mod persistence;` và các module còn lại.
`pub` cho phép truy cập từ bên ngoài module/crate theo phạm vi công khai.
Tên package là `viewer-app`, nhưng trong mã Rust dùng tên `viewer_app`, ví dụ
`use viewer_app::persistence::Persistence;` ở `main.rs`.

### `src/api/mod.rs`: lớp HTTP

| Thành phần | Mục đích |
|---|---|
| `router(database, jvm, auth)` | Tạo router, gắn pool DB/JVM và middleware auth/session dùng chung. |
| `health()` | Trả trạng thái tiến trình đang sống. Không truy vấn DB. |
| `readiness(...)` | Kiểm tra DB và JVM khi bật; thành công trả 200, thất bại trả 503. |
| `Health`, `Readiness` | Kiểu dữ liệu JSON trả ra; derive hỗ trợ JSON và schema. |
| `mod tests` ở cuối tệp | Kiểm thử tích hợp PostgreSQL, migration, transaction, readiness và đóng pool. |

Prefix `/ws/rest` đến từ cấu hình. Các endpoint thực tế là
`/ws/rest/health`, `/ws/rest/ready` và `/ws/rest/docs/openapi.json`.
Readiness có khai báo cả response 200 và 503 cho OpenAPI.

`#[cfg(test)]` chỉ đưa phần test vào bản build kiểm thử. Test PostgreSQL có
`#[ignore]` vì cần DB thật. Chạy riêng test PostgreSQL và bộ JNI bằng các lệnh
trong [tài liệu phát triển](DEVELOPMENT_COMMANDS.md) để cấp đúng runtime.

### `src/persistence/mod.rs`: nền tảng DB

| Thành phần | Mục đích |
|---|---|
| `Persistence` | Giữ connection pool và timeout kiểm tra DB. |
| `connect_from_env()` | Đọc cấu hình, kết nối, SELECT 1 và chạy migration. |
| `connection()` | Cho lớp sử dụng mượn kết nối để truy vấn hoặc bắt đầu transaction. |
| `check()` | Kiểm tra DB bằng SELECT 1 với thời gian chờ giới hạn. |
| `close()` | Đóng pool khi ứng dụng dừng. |
| `Migrator::migrations()` | Migration tạo client/session và permission/client_permission; startup dùng advisory lock trong transaction để tránh DDL đồng thời. |
| `number(...)` | Đọc cấu hình số dương cho pool và timeout. |
| `connection_error(...)` | Trả thông báo lỗi đã lược bỏ thông tin nhạy cảm. |

Đã có entity/repository SeaORM `client`, `session`, `permission`, `client_permission` và migration tương ứng.
Bảng `seaql_migrations` theo dõi migration. Các bảng nghiệp vụ khác được thêm
khi step tương ứng cần dùng.

`database.clone()` chia sẻ pool hiện có, không tạo một pool độc lập cho mỗi lần
clone. `&DatabaseConnection` là tham chiếu mượn kết nối. `Result<T, E>` biểu diễn
kết quả thành công (`Ok`) hoặc lỗi (`Err`); dấu `?` trả lỗi về hàm gọi khi thao
tác thất bại. `impl Persistence` nhóm các hàm gắn với kiểu `Persistence`.

### `auth/`, `domain/`, `dicom/`: ranh giới module

| Module | Trách nhiệm dự kiến | Hiện trạng |
|---|---|---|
| `auth` | Xác thực session, middleware, kiểm tra quyền. | Basic/IP, cache lookup/expiry/share-key, admin/permission guard, CORS và request context. |
| `domain` | Kiểu dữ liệu và quy tắc nghiệp vụ, tránh phụ thuộc framework/I/O. | Principal, role và quy tắc permission snapshot/ADMIN. |
| `dicom` | Ranh giới từ ứng dụng tới crate `dicom-jvm`. | Xuất handle/config an toàn; chưa có thao tác DICOM nghiệp vụ. |

Crate `dicom-jvm` giữ toàn bộ `JNIEnv`/Java reference và worker riêng. Maven module
`java/dicom-facade` cung cấp `apiVersion()` và `capabilitiesJson()` trong JAR đã
đóng gói. Readiness gọi facade qua worker khi JNI bật.

`mod.rs` là tệp định nghĩa module ứng với thư mục chứa nó. Ví dụ
`pub mod auth;` trong `lib.rs` liên kết tới `auth/mod.rs` ở cấu trúc hiện tại.
Các dòng `//!` đầu tệp là chú thích tài liệu cho module.

## 5. Cấu hình và kiểm tra tự động

| Tệp/folder | Vai trò | Khi nào cần đọc/sửa? |
|---|---|---|
| `config/app.toml` | Cấu hình nền: địa chỉ, cổng, prefix HTTP, OpenAPI, logging, graceful shutdown. | Khi thay cấu hình chạy ứng dụng. |
| `config/app-prod.toml` | Ghi đè cho production: bind mặc định ra các interface và log JSON. | Khi thay cấu hình triển khai. |
| `.github/workflows/rust.yml` | CI Windows/Ubuntu: chuẩn bị PostgreSQL riêng, format, lint, test, release build và smoke. | Khi thay quy trình kiểm tra trên GitHub. |
| `tools/smoke_app.py` | Chạy ứng dụng thật ở dev/prod, kiểm tra HTTP/OpenAPI/logging, shutdown và lỗi startup DB. | Khi thay hành vi hạ tầng cần kiểm chứng từ bên ngoài. |
| `tools/baseline_inventory.py` | Đọc repo Java bên cạnh, thu danh sách endpoint và bằng chứng tham chiếu vào `docs/baseline/`. | Khi chủ động cập nhật baseline; chạy lại có thể ghi đè snapshot. |
| `tools/__pycache__/` | Cache Python tự sinh. | Không cần chỉnh sửa hoặc commit. |

`SUMMER_ENV=prod` chọn thêm cấu hình production. `VIEWER_CONFIG` chọn tệp cấu
hình nền; `VIEWER_PORT` và `VIEWER_BIND` đổi cổng/địa chỉ. Các thiết lập DB hiện
được đọc trực tiếp từ môi trường trong `persistence`, không nằm trong TOML.

## 6. Tài liệu trong `docs/`

| Tệp/folder | Nội dung |
|---|---|
| `REPOSITORY_GUIDE.md` | Chính tài liệu đang đọc: bản đồ repo cho người mới. |
| `DEVELOPMENT_COMMANDS.md` | Các lệnh thường dùng trong vòng đời phát triển. |
| `SPRING_TO_RUST_JVM_JNI_MIGRATION_PLAN.md` | Kế hoạch migration hiện hành dùng JVM/JNI. |
| `SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md` | Quyết định và tham chiếu kỹ thuật của kiến trúc hiện hành. |
| `SPRING_TO_RUST_MIGRATION_PLAN.md` | Kế hoạch GraalVM/C ABI cũ, đã được đánh dấu thay thế; chỉ tham khảo lịch sử. |
| `step/README.md` | Mục lục các bước triển khai. |
| `step/step-*.md` | Phạm vi, checklist, kiểm tra và tiến độ riêng của từng bước. Số bước là thứ tự triển khai, không phải module Rust. |
| `baseline/README.md` | Nguồn và giới hạn của bằng chứng thu từ Java. |
| `baseline/inputs/` | Bản chụp tài liệu UC, entity model, sơ đồ và schema Java để đối chiếu. |
| `baseline/source-revision.json` | Nhận diện phiên bản nguồn Java đã khảo sát. |
| `baseline/input-hashes.json` | Hash kiểm tra nội dung các đầu vào baseline. |
| `baseline/endpoint-inventory.md`, `.json` | Danh sách endpoint trích xuất tĩnh; bản dễ đọc và bản máy xử lý. |
| `baseline/configuration-keys.md` | Tên các khóa cấu hình Java đã thu thập. |
| `baseline/uc-checklist.md` | Checklist theo use case để tiếp tục đối chiếu từng tính năng. |

Tài liệu kế hoạch mô tả cả tương lai; hãy xem phần tiến độ từng step và mã
nguồn để biết tính năng nào đã tồn tại. Baseline trích xuất tĩnh không chứng
minh hệ thống đã được kiểm thử runtime.

## 7. `target/`: sản phẩm tự sinh

- `target/debug/`: chương trình và dữ liệu build chế độ phát triển.
- `target/release/`: chương trình và dữ liệu build tối ưu hóa khi dùng `--release`.
- Các thư mục con khác chứa dependency đã biên dịch, mã trung gian và thông tin build.

Trên Windows, chương trình là `viewer-app.exe`; trên Linux là `viewer-app`.
Không sửa mã trong `target/`; sửa trong `crates/viewer-app/src/` rồi build lại.
`target/` không được commit và Cargo có thể tạo lại nó.

## 8. Lộ trình đọc và thực hành

1. Đọc `README.md`, chuẩn bị PostgreSQL và biến môi trường.
2. Đọc `Cargo.toml` ở gốc, rồi `crates/viewer-app/Cargo.toml` để hiểu workspace/package.
3. Đọc `main.rs` để nắm vòng đời ứng dụng.
4. Đọc `lib.rs`, tiếp theo `api/mod.rs` để hiểu module và route.
5. Đọc `persistence/mod.rs`, đối chiếu Step 02 để hiểu kết nối DB và migration.
6. Chạy các lệnh trong `DEVELOPMENT_COMMANDS.md`, quan sát health/readiness.
7. Đọc kế hoạch hiện hành và step tính năng tiếp theo trước khi thêm nghiệp vụ.

Khi muốn sửa endpoint, bắt đầu ở `api`. Khi muốn sửa kết nối/migration, bắt đầu
ở `persistence`. Khi muốn đổi cổng hoặc log, xem `config`. Khi muốn biết một
tính năng đã triển khai đến đâu, xem `docs/step/`.
