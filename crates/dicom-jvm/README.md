# Nền tảng JVM/JNI

Rust không dùng Orthanc transcode/native, khác Java. DICOM giữ nguyên byte.
Tải DICOM/JPEG instance không cần JVM; JPEG lấy frame đã render từ store.
Java chỉ parse metadata/tạo path ZIP cho study/series và encode MP4.

Step 12 bổ sung `download_metadata`, `download_series`, `download_file` với
`FileOperation::Video`.
Queue dùng chung có giới hạn 8,
worker kiểm tra deadline trước khi thực thi và bỏ job có waiter đã đóng. HTTP
timeout/disconnect không giải phóng owner của file/permit khi Java còn chạy.
Mỗi operation dùng local frame; exception được clear trước khi đọc Throwable.
Readiness dùng capabilities đã kiểm chứng khi worker bận, trả lỗi khi call đang
chạy quá deadline; không tạo worker thay thế. JavaCPP cache nằm trong `jni.temp_dir`.
Xem [contract download](../../docs/baseline/step-12-contract.md) và
[cấu hình thử nghiệm](../../config/download.example.toml).

Crate sở hữu một JVM cho mỗi process, sử dụng `jni = 0.21.1` với feature
`invocation`. Phiên bản này được chọn theo API đã đối chiếu và kiểm thử trong
Step 3, không phải tuyên bố là bản mới nhất. Chỉ crate này sử dụng `JNIEnv`,
Java reference và chữ ký method. `viewer-app` nhận handle `Jvm` và dữ liệu Rust.

## Cấu hình

Adapter nhận `JvmSettings` bind từ `[jni]` qua registry TOML/profile của
Summer, sau đó `validate()` thành `JvmConfig`. Không có from_env() hoặc bridge
VIEWER_/PACS_ trong production. Giữ nguyên framework/profile/interpolation;
không sửa default loader hoặc thêm --config. Xem
[contract TOML](../../docs/SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config)
và [file mẫu](../../config/app-dev.example.toml).

| TOML key | Ý nghĩa |
|---|---|
| `jni.enabled` | Boolean, mặc định false. |
| `jni.java_home`, `jni.libjvm_path` | Ít nhất một khi bật; có cả hai phải cùng runtime Java 25. |
| `jni.facade_dir` | Distribution JAR/dependencies/manifest/checksum hợp lệ. |
| `jni.temp_dir`, `jni.native_dir` | Thư mục đã tồn tại; relative path tính từ process cwd như trước. |
| `jni.xms_mb`, `jni.xmx_mb`, `jni.direct_mb` | Số nguyên dương khi bật; Xms ≤ Xmx. |
| `jni.timeout_ms`, `jni.shutdown_ms` | Số nguyên dương khi bật; giữ lifecycle/deadline hiện có. |
| `jni.check_jni` | Default bật debug, tắt release; test đặt true trong TOML. |

Giữ layout Java 25 `bin/server/jvm.dll` trên Windows, `lib/server/libjvm.so`
trên Linux; suy runtime root từ libjvm nếu chỉ cấp đường dẫn library. Không
chọn Java qua PATH. Các biến JVM option `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`,
`JDK_JAVA_OPTIONS` vẫn bị từ chối nếu không rỗng; đây là kiểm tra an toàn môi
trường, không phải nguồn cấu hình ứng dụng. Native loader dùng runtime đã chọn.

Các giá trị heap/direct memory/timeout trong mẫu local là thông số thử nghiệm,
không chốt P1–P4 production. Vendor/build và JRE/jlink production vẫn còn mở.
Không bundle runtime hoặc đổi manifest/capabilities chỉ để chuyển sang TOML.

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

Script ghi cấu hình JNI thử nghiệm vào TOML, libtest nhận đường dẫn fixture
qua JNI_TEST_CONFIG (test-only). App subprocess dùng config/app.toml + overlay
trong working directory tạm, không cấp settings bằng env ứng dụng. JNI_TEST_CASE
chỉ điều khiển fixture lỗi Java. PATH/JAVA_HOME cho compiler/runtime phụ thuộc và
việc làm sạch JAVA_TOOL_OPTIONS vẫn thuộc môi trường công cụ, không phải nguồn
settings production. Mỗi JVM chạy trong subprocess riêng với deadline bên ngoài;
runner luôn bật -Xcheck:jni. Không đóng gói artifact gây lỗi vào facade production.

Log ở `target/step-03-logs/`: `jni-suite.log`, `jni-<case>.log`, `jni-readiness.log`,
`app-<profile>-jni.log`, `app-fault-<case>.log`. Bộ test kiểm tra exception recovery,
attach/detach, local frame, Tokio vẫn tiến triển, lỗi runtime/artifact/contract,
HTTP readiness và thoát process khi Java call/thread treo. Từ Step 12, case
`download` kiểm tra queue/deadline, cancellation/file ownership và reference loop
với video fixture. MP4 thật chạy riêng bằng `smoke_download.py`; JPEG HTTP được
kiểm tra với fixture nhiều frame và store thật bằng `verify_live_download.py`.
Printer không thuộc phạm vi kiểm chứng này.

Nguồn đối chiếu:
[jni-rs 0.21.1 JavaVM](https://docs.rs/jni/0.21.1/jni/struct.JavaVM.html),
[JNIEnv](https://docs.rs/jni/0.21.1/jni/struct.JNIEnv.html),
[Java 25 JNI exception handling](https://docs.oracle.com/en/java/javase/25/docs/specs/jni/design.html#exception-handling).
