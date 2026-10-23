# Step 3 - Khởi tạo JVM/JNI và facade tối thiểu

## Tính năng

Sau khung Rust/DB, tạo kết nối JVM 25 và facade không Spring. Chưa đối chiếu hoặc triển khai toàn bộ parse/render/transcode/key-image/print.

**JNI:** Có — JVM startup và facade smoke test.

## Nguồn Java

- [pom.xml Java](../../../1pacs-viewer-core/pom.xml): chỉ lấy dependency DICOM hiện tại.
- [Quyết định T7–T13](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions).

## Việc cần làm

- [ ] Khóa jni-rs, tạo facade JAR + dependency directory; load đúng libjvm Java 25 và tạo một JVM/process.
- [x] Facade có apiVersion/capabilitiesJson; chỉ công bố operation/codec đã thực sự triển khai và kiểm chứng.
- [ ] Smoke call qua worker JNI riêng, attach/detach đúng thread và local frame; không block Tokio.
- [ ] Xử lý exception theo [mục 5.5](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-exceptions); dùng lỗi tổng hợp để kiểm tra.
- [ ] Khi JNI bật, readiness kiểm tra JVM/facade version; startup failure rõ ràng, shutdown có giới hạn; lỗi không phục hồi restart process.
- [ ] Chuyển worker scheduling/byte[]/file ownership và DICOM operation sang step đầu tiên sử dụng chúng; không chạy golden corpus/print suite trước khi có tính năng.

## Kiểm tra hoàn thành

- [ ] Smoke test JVM/facade đúng bản đóng gói; sai path/classpath/version bị từ chối.
- [ ] Kiểm tra exception, local reference và attach/detach/shutdown dưới -Xcheck:jni, chưa cần DICOM corpus hoặc printer.
- [ ] Chỉ đóng foundation; byte/file transport, concurrency, cancellation, codec và print được kiểm thử tại step sử dụng và hồi quy ở Step 18.

## Tiến độ

- Trạng thái: `in_progress`
- Đã làm: Hoàn tất đầu việc 1 — Maven module Java 25 độc lập tại `java/dicom-facade`, thin JAR, thư mục dependency rỗng, manifest/checksum và smoke probe chỉ nạp class từ packaged JAR. `apiVersion()` trả 1; `capabilitiesJson()` công bố operations/codecs rỗng. Có lệnh build/verify và log theo dõi. Chưa triển khai Rust/JNI.
- Tiếp theo: Khi được yêu cầu tiếp tục, khóa dependency JNI sau khi đối chiếu API chính thức, tạo adapter rồi tích hợp lifecycle/readiness. Checklist đầu tiên vẫn chưa hoàn thành vì chưa load libjvm hoặc tạo JVM từ Rust.
- Vướng mắc: Vendor/build và hình thức runtime production chưa chốt; không cản trở implementation và thử nghiệm local có ghi rõ runtime.
- Kiểm tra gần nhất: 2026-10-23 — `python tools/facade.py build --java-home "C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot" --maven-repo "C:\Users\Admin\.m2\repository"` và `python tools/facade.py verify --java-home "C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot"` đạt. Runtime Eclipse Adoptium 25.0.4.1+1-LTS, Windows amd64; Maven 3.9.11. Packaged smoke chạy với `-Xcheck:jni`, kiểm tra code source JAR, API và JSON. Kiểm tra trên bản sao artifact xác nhận từ chối JAR bị sửa, JAR bị thiếu và manifest sai API version. `git diff --check` đạt.
- Evidence local: `target/step-03-logs/build-facade.log`, `facade-toolchain.log`, `compile-facade-smoke.log`, `smoke-facade.log`, `negative-facade.log`; artifact ở `java/dicom-facade/target/distribution/`. Log/artifact không commit. Maven test phase chưa có unit test; bằng chứng là packaged smoke của lệnh Python. Chưa có JNI integration test hoặc kết quả Linux/production.
- Cập nhật: 2026-10-23 — Codex hoàn tất đầu việc facade theo yêu cầu người dùng; chưa commit vì Step 3 còn tiếp tục.

## Đầu vào đã kiểm tra

- [Step 2](step-02-persistence.md) đã `done` trên Windows local; CI Windows/Ubuntu chưa có kết quả được ghi nhận.
- Workspace hiện có `crates/viewer-app`; module `dicom` mới là vị trí dành cho adapter. Java facade đã được thêm trong đầu việc 1; chưa có crate JNI.
- `main.rs` kết nối DB và chạy migration trước HTTP, đóng DB sau khi server/scheduler dừng. `/ws/rest/ready` hiện chỉ kiểm tra PostgreSQL; `/ws/rest/health` kiểm tra tiến trình sống.
- `pom.xml` Java dùng dcm4che **5.33.1**. Chỉ đưa dependency cần thiết vào facade; không sao chép Spring parent hoặc toàn bộ dependency backend.
- Runtime local quan sát được: Eclipse Temurin **25.0.4.1+1-LTS**, Windows, tại `C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot`. Đây là môi trường thử nghiệm, chưa phải lựa chọn runtime production.
- Giữ nguyên thay đổi của công việc khác khi triển khai Step 3; không đưa chúng vào commit của step.

## Trình tự triển khai dự kiến

### 1. Facade và bản đóng gói

- Tạo Maven module độc lập tại `java/dicom-facade`, target Java 25; facade class `vn.saolasoft.pacs.dicom.DicomFacade`.
- Contract foundation đã triển khai: `public static int apiVersion()` trả `1`; `public static String capabilitiesJson()` trả JSON với `apiVersion: 1`, `operations: []`, `codecs: []`. Rust sẽ kiểm tra kiểu dữ liệu và tính nhất quán giữa hai kết quả khi triển khai adapter. Chưa công bố parse/render/transcode/print chỉ vì JAR dependency đã tồn tại.
- Hai method trên là smoke call; chưa tạo DTO nghiệp vụ hoặc stub operation DICOM. Signature/DTO của từng operation được xác định tại step sử dụng đầu tiên theo mục 19 của tham chiếu kỹ thuật.
- Build thin JAR `pacs-dicom-facade.jar` và `dependencies/`; ghi manifest/checksum, facade API/build và runtime dùng thử. Dependency DICOM được thêm khi có thao tác cần kiểm chứng; giữ baseline 5.33.1.
- Synthetic exception và facade sai version/signature nằm trong artifact test riêng; không thêm method gây lỗi vào facade production chỉ để phục vụ test.

### 2. Rust adapter và cấu hình runtime

- Tạo crate `crates/dicom-jvm`, được `viewer-app::dicom` sử dụng; giữ mọi JNI type, reference và signature trong crate này. Chỉ tạo thêm crate khác khi có trách nhiệm thực tế, không tạo tầng raw JNI chưa cần thiết.
- Đối chiếu tài liệu jni-rs theo phiên bản, chọn feature Invocation API và khóa phiên bản thực tế trong Cargo.lock. Chưa coi một phiên bản crate là đã chọn trong giai đoạn chuẩn bị.
- Đề xuất cấu hình bật/tắt bằng `VIEWER_JNI_ENABLED`, mặc định tắt để ứng dụng nền tảng chạy được khi chưa dùng DICOM. Khi bật, bắt buộc runtime explicit qua `PACS_JAVA_HOME` hoặc `PACS_LIBJVM_PATH`; nếu cấp cả hai phải kiểm tra chúng chỉ cùng runtime. Không fallback sang Java trên PATH.
- Bổ sung cấu hình đường dẫn facade/dependency, temp/native directory và giới hạn JVM; xác thực trước khi tạo JVM. Classpath dùng dấu phân cách của OS, kiểm tra artifact/manifest và Java major version 25.
- Ghi riêng các giá trị heap/direct memory, timeout và giới hạn thử nghiệm vào evidence. P1–P4 vẫn chưa chốt cho production; không biến giá trị thử thành quyết định đã duyệt.

### 3. Lifecycle và smoke call

- Trước khi mở HTTP: DB/migration → load đúng libjvm → tạo một JVM → attach worker JNI riêng → resolve facade → kiểm tra version/capabilities → cho phép readiness.
- Dùng worker thread riêng có attach guard; mỗi job có local frame giới hạn. Giao tiếp bất đồng bộ với Tokio, không chuyển `JNIEnv` qua thread. Chỉ cần kênh có giới hạn cho các lệnh foundation; worker scheduling cho nghiệp vụ để step sử dụng đầu tiên thiết kế.
- Thực hiện thứ tự lấy Throwable → clear pending exception → đọc thông tin giới hạn → map lỗi theo mục 5.5. Nếu việc đọc Throwable lỗi tiếp, clear và dùng fallback; không ghi exception thô hoặc đường dẫn nhạy cảm vào log/HTTP.
- JVM/classpath/API/capability startup mismatch phải kết thúc startup với exit code lỗi. Readiness khi JNI bật kết hợp DB và trạng thái JVM/facade, trả 503 nếu dependency lỗi; khi tắt chỉ kiểm tra DB. Health giữ ý nghĩa tiến trình sống.
- Khi dừng: ngừng nhận việc JNI, chờ worker trong thời gian giới hạn, giải phóng global reference và detach trên đúng thread. Không đặt lời gọi hủy JVM có thể block trên Tokio. Nếu không kết thúc an toàn trong deadline hoặc JVM không phục hồi, kết thúc process theo T13; không thử tạo JVM thay thế trong cùng process.

### 4. Kiểm thử và tài liệu vận hành

- Build Java facade trước Rust integration tests; test từ JAR/dependency directory thực tế, không dùng class directory của Maven làm bằng chứng đóng gói.
- Mỗi kịch bản JVM chạy trong subprocess riêng vì chỉ tạo một JVM/process. Harness có deadline bên ngoài để phát hiện startup/shutdown bị treo.
- Bổ sung Java 25 và build artifact vào CI Windows/Ubuntu; ghi vendor/build và libjvm thực tế. CI dùng JDK chưa thay thế bằng chứng runtime production đã đóng gói; xác minh distribution production sau khi chốt T7 bổ sung.
- Cập nhật README, hướng dẫn lệnh, smoke harness và tiến độ bằng kết quả đã chạy. Step này không thêm endpoint DICOM hoặc bảng nghiệp vụ.

## Ma trận kiểm thử foundation

| Nhóm | Tình huống | Kết quả cần chứng minh |
|---|---|---|
| JNI tắt | DB hợp lệ, không cấp runtime Java | App chạy; readiness vẫn dựa trên PostgreSQL |
| Startup hợp lệ | Runtime Java 25 explicit + packaged facade/dependency | Một JVM được tạo; API/capabilities khớp; readiness 200 khi DB tốt |
| Runtime lỗi | Thiếu/sai libjvm, Java không phải 25, cấu hình mâu thuẫn | Startup lỗi trước khi nhận HTTP; thông báo an toàn |
| Artifact lỗi | Thiếu/sai JAR, checksum, classpath, class hoặc method/signature | Startup bị từ chối, không công bố readiness thành công |
| Contract lỗi | Sai API version, JSON capabilities sai hoặc mâu thuẫn | Startup bị từ chối với mã lỗi ổn định |
| Java exception | Synthetic Throwable; lỗi tiếp khi lấy thông tin Throwable | Pending exception được clear; fallback an toàn; smoke call sau vẫn thành công |
| Reference/thread | Lặp smoke call trong local frame với `-Xcheck:jni` | Không cảnh báo JNI; reference không tích lũy theo số call; attach/detach đúng thread |
| Tokio | Worker xử lý smoke call trong khi runtime chạy tác vụ khác | JNI không chạy trên Tokio worker, runtime vẫn phục vụ tác vụ khác |
| Readiness | DB lỗi hoặc worker/JVM được đánh dấu không phục hồi | Readiness 503 trong thời gian process còn chạy; lỗi fatal kết thúc process |
| Shutdown | Worker bình thường và test Java call/thread cố tình treo | Bình thường giải phóng/detach; treo kết thúc process trong giới hạn harness, không restart JVM in-process |
| Hồi quy | Format/clippy/tests/build và smoke dev/prod của Step 1–2 | DB bắt buộc, migration, HTTP/OpenAPI và shutdown hiện có vẫn đúng |

Khi ghi evidence, phân biệt rõ kiểm chứng local Windows, CI Linux và distribution production. Không đánh dấu checklist hoàn thành từ thiết kế hoặc chỉ từ unit test mock. DICOM corpus, byte/file ownership, cancellation nghiệp vụ, codec và printer suite thuộc step sử dụng đầu tiên và hồi quy Step 18.
