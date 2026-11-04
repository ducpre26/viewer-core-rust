# JVM facade

Thư viện Java 25 độc lập cho nền tảng Step 3 và download Step 12. API công khai:
`vn.saolasoft.pacs.dicom.DicomFacade`.

| Phương thức | Chữ ký JNI | Kết quả |
|---|---|---|
| `public static int apiVersion()` | `()I` | `2` |
| `public static String capabilitiesJson()` | `()Ljava/lang/String;` | API 2; downloadMetadata/downloadSeries/encodeVideo; codecs chưa quảng bá |
| `downloadMetadata(json, pattern, index)` | `(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;` | Metadata và path theo dcm4che AttributesFormat |
| `downloadSeries(json)` | `(Ljava/lang/String;)Ljava/lang/String;` | UID/index series theo baseline Java |
| `encodeVideo(manifest, output, fps)` | `(Ljava/lang/String;Ljava/lang/String;I)V` | JavaCV/FFmpeg MP4, xử lý từng frame |

API 2 bỏ quota download và lớp LimitedOutput, không tương thích chữ ký API 1.
Build/deploy distribution và binary Rust cùng nhau. Không còn guard metadata
16 MiB; thư viện/JVM vẫn có giới hạn kiểu dữ liệu và bộ nhớ thực tế.

Phiên bản API độc lập với phiên bản gói thư viện `0.1.0`. Step 12 dùng dcm4che
5.33.1, JavaCV 1.5.9 và FFmpeg 6.0-1.5.9 theo baseline. Rust không dùng Orthanc transcode/native, khác Java. DICOM giữ
nguyên byte ở Rust; JPEG lấy từ WADO-RS rendered của store. Facade chỉ parse
metadata/tạo path và encode MP4. Native FFmpeg vẫn được đóng gói cho Windows/Linux
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
