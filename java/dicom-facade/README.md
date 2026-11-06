# JVM facade

Thư viện Java 25 độc lập cho nền tảng Step 3, download Step 12 và key image Step 13. API công khai:
`vn.saolasoft.pacs.dicom.DicomFacade`.

| Phương thức | Chữ ký JNI | Kết quả |
|---|---|---|
| `public static int apiVersion()` | `()I` | `2` |
| `public static String capabilitiesJson()` | `()Ljava/lang/String;` | API 2; downloadMetadata/downloadSeries/encodeVideo/keyImage; codecs chưa quảng bá |
| `downloadMetadata(json, pattern, index)` | `(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;` | Metadata và path theo dcm4che AttributesFormat |
| `downloadSeries(json)` | `(Ljava/lang/String;)Ljava/lang/String;` | UID/index series theo baseline Java |
| `encodeVideo(manifest, output, fps)` | `(Ljava/lang/String;Ljava/lang/String;I)V` | JavaCV/FFmpeg MP4, xử lý từng frame |
| `keyImage(operation, manifest)` | `(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;` | Parse/write/merge/frame theo Java; Rust giữ session, pending và STOW |

API 2 bỏ quota download và lớp LimitedOutput, không tương thích chữ ký API 1.
Build/deploy distribution và binary Rust cùng nhau. Không còn guard metadata
16 MiB; thư viện/JVM vẫn có giới hạn kiểu dữ liệu và bộ nhớ thực tế.

Phiên bản API độc lập với phiên bản gói thư viện `0.1.0`. Step 12 dùng dcm4che
5.33.1, JavaCV 1.5.9 và FFmpeg 6.0-1.5.9 theo baseline. Rust không dùng Orthanc transcode/native, khác Java. DICOM giữ
nguyên byte ở Rust; JPEG lấy từ WADO-RS rendered của store. Facade parse metadata/tạo path, encode MP4 và xử lý DICOM key image;
việc lấy JPEG từ store chỉ áp dụng download, không giới hạn ingest key image. Native FFmpeg vẫn được đóng gói cho Windows/Linux
x64 theo T14; các method facade không tự gọi HTTP hoặc ZIP.
Module không kế thừa Spring
parent, không chạy server, truy cập database hoặc thêm điểm móc kiểm thử vào mã production.

Chạy từ thư mục gốc repo và chỉ định rõ JDK 25:

```powershell
python tools/facade.py build --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
python tools/facade.py verify --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
```

Trên Linux, dùng cùng các lệnh trên với đường dẫn JDK của máy. Nếu không truy cập
được cache mặc định của Maven, thêm `--maven-repo <directory>` vào lệnh build.
Cần Python 3 và Maven 3.9. Script thiết lập `JAVA_HOME` và `PATH` cho các tiến trình
con để Maven và smoke test dùng đúng JDK đã chỉ định. Các tùy chọn khởi chạy Java
và `CLASSPATH` có sẵn trong môi trường được loại khỏi những tiến trình kiểm tra này.

Lệnh `build` chạy Maven `clean package`, tạo thư mục dependency,
biên dịch chương trình kiểm tra riêng dựa trên JAR đã đóng gói, rồi chạy trong một
tiến trình Java mới với `-Xcheck:jni`. Chương trình kiểm tra xác nhận Java 25,
vị trí JAR chứa class được nạp và phiên bản API. Python phân tích và kiểm tra JSON
trả về. Đây là smoke test cho bản đóng gói Java; chưa kiểm chứng hành vi Rust/JNI.
Giai đoạn test của Maven chưa có unit test; smoke test từ JAR được chạy bằng lệnh Python.

Kết quả nằm trong `java/dicom-facade/target/distribution/`:

```text
pacs-dicom-facade.jar
dependencies/
manifest.json
```

Manifest JSON ghi phiên bản API/gói thư viện, runtime thực tế đã kiểm tra, commit
nguồn và trạng thái có thay đổi chưa commit, cùng mã băm SHA-256 của tất cả JAR.
Lệnh `verify` kiểm tra manifest, danh sách JAR và checksum trước khi chạy lại smoke
test; lệnh này không tạo lại manifest. Manifest cung cấp thông tin kiểm tra tính
toàn vẹn, không phải chữ ký số hoặc bằng chứng runtime đã được duyệt cho production.
JDK chưa được đóng gói kèm theo.

Log được ghi đè theo từng lệnh trong `target/step-03-logs/`: `build-facade.log`,
`facade-toolchain.log`, `compile-facade-smoke.log`, `smoke-facade.log`.
Thời gian chờ tối đa: build 10 phút, biên dịch chương trình kiểm tra 2 phút,
chạy chương trình kiểm tra 30 giây. Đây là giới hạn build/test local, chưa phải
các quyết định P1–P4 cho production.

Phiên bản plugin Maven được ghim cố định. Cấu hình đóng gói dựa trên tài liệu chính thức về
[chọn phiên bản Java khi biên dịch](https://maven.apache.org/plugins/maven-compiler-plugin/examples/set-compiler-release.html),
[sao chép dependency runtime](https://maven.apache.org/plugins/maven-dependency-plugin/examples/copying-project-dependencies.html)
và [các mục trong manifest JAR](https://maven.apache.org/shared/maven-archiver/examples/manifestEntries.html).


Step 13 giữ thứ tự Java: `parse` ghi metadata/bulk manifest vào thư mục job;
Rust validate/tạo pending; `write` bổ sung thuộc tính và ghi DICOM/metadata.
`merge` giữ dcm4che JSON/HashMap khi ghép dữ liệu local; `frame` ghi multipart
JPEG local vào file job. Manifest/path do Rust tạo; worker giữ file lease và
lock storage đến hết call, kể cả HTTP timeout/disconnect. Không có Java reference
thoát boundary. Method trả JSON dataset hoặc lỗi có status/errorCode/message;
JNI exception bất ngờ vẫn qua cơ chế clear/withhold dùng chung.

Các helper DICOM được port từ revision Java `2373896f`; không chứa Spring,
repository, scheduler hoặc HTTP client. Dependency thêm từ Java nguồn:
dcm4che-mime/ws-rs 5.33.1, jersey-common 3.1.10, commons-io 2.15.1.
Nhánh QuickTime của nguồn vẫn gọi `ffmpeg` để đổi container; môi trường dùng
nhánh này cần cung cấp executable đó. Corpus đã kiểm chứng và giới hạn bằng
chứng ghi ở Step 13; không suy diễn hỗ trợ codec chưa thử trên production.
Đóng stream metadata để không giữ khóa file Windows và kiểm tra containment
đường dẫn là trách nhiệm tài nguyên của facade, không thay thứ tự pending/cleanup.
