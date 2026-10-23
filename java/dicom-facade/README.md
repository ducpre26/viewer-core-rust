# JVM facade tối thiểu

Thư viện Java 25 độc lập phục vụ Step 3. Lớp cung cấp API công khai:
`vn.saolasoft.pacs.dicom.DicomFacade`.

| Phương thức | Chữ ký JNI | Kết quả |
|---|---|---|
| `public static int apiVersion()` | `()I` | `1` |
| `public static String capabilitiesJson()` | `()Ljava/lang/String;` | `{"apiVersion":1,"operations":[],"codecs":[]}` |

Phiên bản API độc lập với phiên bản gói thư viện `0.1.0`. Hiện chưa có dependency
runtime, thao tác DICOM hoặc codec. Dependency được bổ sung tại step cần sử dụng;
phiên bản dcm4che làm cơ sở đối chiếu vẫn là 5.33.1. Module không kế thừa Spring
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

Lệnh `build` chạy Maven `clean package`, tạo thư mục dependency (hiện đang rỗng),
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
