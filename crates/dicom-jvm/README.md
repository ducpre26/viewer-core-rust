# Nền tảng JVM/JNI

Crate sở hữu một JVM cho mỗi process, sử dụng `jni = 0.21.1` với feature
`invocation`. Phiên bản này được chọn theo API đã đối chiếu và kiểm thử trong
Step 3, không phải tuyên bố là bản mới nhất. Chỉ crate này sử dụng `JNIEnv`,
Java reference và chữ ký method. `viewer-app` nhận handle `Jvm` và dữ liệu Rust.

## Cấu hình

JNI mặc định tắt. Khi bật, các cấu hình bắt buộc dưới đây phải được cấp từ môi trường:

| Biến | Ý nghĩa |
|---|---|
| `VIEWER_JNI_ENABLED` | `true` hoặc `false`; mặc định `false`. |
| `PACS_JAVA_HOME` / `PACS_LIBJVM_PATH` | Cấp ít nhất một đường dẫn. Nếu cấp cả hai phải trỏ cùng runtime. |
| `PACS_FACADE_DIR` | Thư mục chứa JAR, `dependencies/`, `manifest.json`. |
| `PACS_JVM_TEMP_DIR` | Thư mục tạm đã tồn tại, dành riêng cho JVM. |
| `PACS_JVM_NATIVE_DIR` | Thư mục native library đã tồn tại; foundation có thể để rỗng. |
| `PACS_JVM_XMS_MB`, `PACS_JVM_XMX_MB` | Heap khởi tạo/tối đa, số nguyên dương; Xms không vượt Xmx. |
| `PACS_JVM_DIRECT_MB` | Giới hạn direct memory, số nguyên dương. |
| `PACS_JVM_TIMEOUT_MS` | Thời hạn chờ startup hoặc kết quả kiểm tra facade. |
| `PACS_JVM_SHUTDOWN_MS` | Thời hạn chờ worker detach và JVM kết thúc. |
| `PACS_JVM_CHECK_JNI` | Bật `-Xcheck:jni`; mặc định bật trong debug, tắt trong release. Bộ test luôn bật. |

Adapter hỗ trợ layout Java 25 `bin/server/jvm.dll` trên Windows và
`lib/server/libjvm.so` trên Linux. Với `PACS_LIBJVM_PATH` đơn lẻ, runtime root
được suy ra từ layout này. Không dò Java qua PATH. Trên Windows, thêm thư mục
`bin` của JDK đã chọn vào PATH trước khi chạy để hệ điều hành tìm được DLL phụ thuộc.
Các biến `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS` phải rỗng hoặc
không có; adapter từ chối chúng để tránh thêm tùy chọn JVM ngoài cấu hình.

Ví dụ **chỉ dành cho thử nghiệm local**, sau khi build facade và cấp cấu hình DB:

```powershell
$jdk = 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
New-Item -ItemType Directory -Force target/jni-temp, target/jni-native | Out-Null
$env:VIEWER_JNI_ENABLED = 'true'
$env:PACS_JAVA_HOME = $jdk
$env:PATH = "$jdk\bin;$env:PATH"
$env:PACS_FACADE_DIR = 'java/dicom-facade/target/distribution'
$env:PACS_JVM_TEMP_DIR = 'target/jni-temp'
$env:PACS_JVM_NATIVE_DIR = 'target/jni-native'
$env:PACS_JVM_XMS_MB = '32'
$env:PACS_JVM_XMX_MB = '128'
$env:PACS_JVM_DIRECT_MB = '32'
$env:PACS_JVM_TIMEOUT_MS = '5000'
$env:PACS_JVM_SHUTDOWN_MS = '1500'
$env:PACS_JVM_CHECK_JNI = 'true'
cargo run --locked -p viewer-app
```

Các giá trị trên không chốt P1–P4 cho production. Runtime vendor/build và JRE/jlink
production vẫn còn mở. Distribution hiện chứa facade/dependency và manifest;
Java runtime được chỉ định từ bên ngoài, chưa được bundle.

## Vòng đời và giới hạn

1. Kiểm tra runtime Java 25, manifest API/schema, inventory JAR và SHA-256.
   Classpath chỉ chứa các JAR đã kiểm tra; không dùng Maven `target/classes`.
2. Worker `dicom-jni` nạp đúng libjvm và tạo JVM. Cờ toàn process ngăn lần khởi tạo
   thứ hai, kể cả sau lỗi hoặc shutdown; không restart JVM trong cùng process.
3. Worker attach một lần bằng scoped guard, giữ facade class bằng global reference.
   Mỗi kiểm tra chạy trong local frame dung lượng 32, kiểm tra Java major version
   và facade API/capabilities. JSON capabilities giới hạn 4096 ký tự trước khi copy.
4. HTTP readiness gọi worker qua kênh có giới hạn 8 lệnh điều khiển. Chưa có pool
   cho nghiệp vụ DICOM. Kênh đầy trả lỗi busy; lệnh đã hủy khi còn trong queue được bỏ qua.
   JNI chạy đồng bộ trên worker riêng, không chạy trên thread Tokio.
5. Exception được lấy reference rồi clear trước khi kiểm tra loại/message. Nội dung
   message không được copy ra Rust hoặc ghi log. Nếu đọc message sinh exception
   thứ hai, clear rồi trả mã fallback cố định; không gọi `ExceptionDescribe`.
6. Khi dừng HTTP/scheduler, worker giải phóng global reference khi còn attached,
   drop guard để detach, rồi gọi `DestroyJavaVM` trên chính worker. Tokio chỉ chờ
   tín hiệu hoàn thành có timeout. Không còn Rust daemon attachment hoặc JNI handle
   được dùng sau destroy; đây là điều kiện an toàn của khối `unsafe` duy nhất.

Startup thất bại không mở HTTP. Readiness trả 503 khi dependency lỗi; lỗi JVM fatal
có thể đóng kết nối trước khi response 503 được gửi xong vì ứng dụng phải thoát.
Timeout của call đang chạy không tự hủy Java: handle bị đánh dấu lỗi và ứng dụng
thoát với mã 1 để hệ thống vận hành restart process. Shutdown quá hạn cũng thoát
mã 1. Khi dừng bình thường, JVM kết thúc trước khi đóng pool PostgreSQL.

Mã lỗi cố định gồm `JVM_CONFIG`, `JVM_VERSION`, `JVM_ARTIFACT`, `JVM_CONTRACT`,
`JVM_STARTUP`, `JVM_ONCE`, `JNI_CALL`, `JAVA_EXCEPTION`, `JAVA_EXCEPTION_FALLBACK`,
`JVM_FATAL`, `JVM_BUSY`, `JVM_TIMEOUT`, `JVM_STOPPED`, `JVM_SHUTDOWN`.
Log không chứa exception thô, classpath hoặc credential.

## Kiểm thử

```powershell
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
# Cần PostgreSQL riêng và binary viewer-app đã build:
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot' --app
# Sau cargo build --workspace --release --locked:
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot' --release --app
```

Script cấp các giá trị thử nghiệm ở ví dụ trên, làm sạch tùy chọn Java kế thừa,
và luôn bật `-Xcheck:jni`. Mỗi kịch bản chạy trong subprocess riêng có deadline
bên ngoài 30 giây. Fixture gây lỗi nằm trong `tests/fixtures`, được build thành
JAR riêng khi test và không đóng gói vào facade production.

Log ở `target/step-03-logs/`: `jni-suite.log`, `jni-<case>.log`, `jni-readiness.log`,
`app-<profile>-jni.log`, `app-fault-<case>.log`. Bộ test kiểm tra exception recovery,
attach/detach, local frame, Tokio vẫn tiến triển, lỗi runtime/artifact/contract,
HTTP readiness và thoát process khi Java call/thread treo. Không kiểm tra DICOM
corpus, codec, byte/file ownership hay printer trong foundation.

Nguồn đối chiếu:
[jni-rs 0.21.1 JavaVM](https://docs.rs/jni/0.21.1/jni/struct.JavaVM.html),
[JNIEnv](https://docs.rs/jni/0.21.1/jni/struct.JNIEnv.html),
[Java 25 JNI exception handling](https://docs.oracle.com/en/java/javase/25/docs/specs/jni/design.html#exception-handling).
