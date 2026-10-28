# Step 12 - UC-005 Tải nội dung DICOM

## Tính năng

Download DICOM/JPEG theo study, series, instance; MP4 theo series.
Rust xử lý HTTP, ZIP, file tạm và giới hạn tài nguyên. Java qua JNI xử lý metadata,
tên đường dẫn ZIP và encode MP4 bằng JavaCV/FFmpeg theo quyết định T14.

**Khác Java:** Rust không dùng Orthanc transcode/native; DICOM giữ nguyên byte,
JPEG lấy từ WADO-RS rendered của store. Repo Java gốc không đổi.

## Nguồn Java

- [DownloadAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/rest/DownloadAPI.java)
- [DownloadServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/service/DownloadServiceImpl.java)
- [UC-005](../../../1pacs-viewer-core/docs/use_cases/UC-005-tai-noi-dung-dicom.md)
- [Contract và khác biệt đã chốt](../baseline/step-12-contract.md), [baseline 24 case](../baseline/step-12-java.json)

## Việc cần làm

- [x] Facade metadata/video; JNI queue, deadline và ownership cho video.
- [x] Đối chiếu Java, revision, fixture và expected result tại contract.
- [x] Tái sử dụng SESSION/auth và CONFIG `DOWNLOAD_PATH_FORMAT`; không thêm schema/migration.
- [x] Ba route và phạm vi DICOM/JPEG/MP4; kiểm chứng output theo bảng bằng chứng bên dưới.
- [x] Từ chối kiểu tải không hỗ trợ, giữ 415/errorCode theo baseline.
- [x] ZIP structure/naming, chống path không an toàn/tên trùng; HTTP stream file 64 KiB.
- [x] DICOM nguyên byte; JPEG WADO-RS rendered, frame 1-based, quality=90.
- [x] T14: giữ Java FFmpeg binding cho MP4.
- [x] Cancellation giữ file/permit đến completion; file lock bảo vệ trước TTL cleanup.

## Kiểm tra hoàn thành

- [ ] Corpus JNI/codec đủ phạm vi; queue/deadline/cancellation/reference đã có test, corpus rộng còn hoãn.
- [ ] API/output và tác động DB/file khớp baseline trong phạm vi đã chốt; còn xác nhận lỗi với dữ liệu store thật.
- [ ] UC-005 main flow, A1..A3, BR-001..BR-003; golden ZIP/JPEG/MP4 đã có, disk-full và memory peak còn hoãn.
- [x] Cancellation/disconnect khi FFmpeg thật đang encode: file/permit hợp lệ, TTL bỏ qua job active, cleanup sau completion. Input synthetic, chưa xác nhận với dữ liệu STORE.md.
- [ ] Output mở được bằng tool độc lập đã đạt trên mẫu hiện có; corpus codec và resource limits production chưa hoàn tất.

## Tiến độ sau review — 2026-10-28

- Trạng thái: `in_progress`. Người dùng xác nhận commit phần đã làm ngày
  2026-10-28; các mục chưa kiểm chứng bên dưới vẫn giữ nguyên.
- Đã triển khai: routes, auth/session, metadata/path, ZIP, JPEG từ store, MP4 qua JNI,
  typed TOML, quota, semaphore, deadline, lease/TTL và file response có Content-Length.
- Đã sửa qua test lỗi: store trả non-200 bị chuyển thành DICOM attachment 200;
  hiện trả lỗi đã lược bỏ chi tiết nguồn trước khi gửi file.
- Cần xác nhận tiếp với STORE.md: MP4 dùng dữ liệu thật, hủy khi đang encode;
  lỗi tải và thời điểm response qua proxy cục bộ chủ động ngắt/chặn nguồn thật.
  Chưa chạy các bài này; 26 bài giả lập không thay bằng chứng store thật.
- Ghi để test sau theo yêu cầu: corpus transfer syntax/color/compressed rộng,
  đĩa thật đầy, series lớn/memory peak/soak, release Windows và Linux/CI.
- Còn quyết định cấu hình production P1–P4; download mặc định tắt, giới hạn trong
  `config/download.example.toml` chỉ dành cho thử nghiệm.
- Lượt review này chỉ chỉnh tài liệu/tiến độ, không chạy thêm test runtime.

## Bằng chứng đã có

| Kiểm tra | Nguồn dữ liệu / môi trường | Kết quả và giới hạn |
|---|---|---|
| DICOM/JPEG instance; ZIP study/series | Store thật trong STORE.md: 1 series, 2 instance | Byte khớp nguồn; JPEG giải mã được, DICOM giữ JPEG Baseline. Instance chạy JNI tắt; ZIP dùng Java metadata thật |
| MP4 đối chiếu Java | Synthetic 2 frame 16×16; JavaCV trực tiếp và Rust/JNI thật | FFmpeg độc lập giải mã, pixel khớp Java gốc; chưa đại diện corpus rộng |
| Route/header/naming/auth/ZIP | 24 case Java baseline, HTTP store giả; video double trong HTTP contract | Đạt phạm vi fixture; byte DICOM/JPEG theo khác biệt đã chốt |
| Lỗi store và thời điểm response | Store giả localhost, backend production | 26 trường hợp đạt; không phải 26 test trên STORE.md |
| Hủy/deadline trong lúc encode | Synthetic 384 frame 512×512, JavaCV/FFmpeg thật | MP4 giải mã đủ frame; file/permit giữ tới completion, TTL bỏ qua active, readiness phục hồi |
| JNI queue/deadline/reference | JVM thật, codec double cho bài điều phối | Đạt suite đã chạy; không thay kiểm chứng mọi codec |
| Workspace | Debug Windows | 20 pass, 14 ignored không tính đạt; JNI/video/HTTP liên quan chạy riêng |
| Hồi quy DB dùng chung | PostgreSQL theo TOML | 9 test auth/config/session/study/WADO đã đạt ở lượt trước |
| Build/lint | Debug Windows | Build, clippy -D warnings, format và diff check đạt ở lượt triển khai gần nhất |

Bằng chứng cục bộ:

- `target/step-12-logs/live-store/result.json`: store thật, byte/decoded JPEG và ZIP.
- `target/step-12-logs/mp4-independent.json`, `jni-real-video.log`, `http-download-contract.log`: MP4 và contract HTTP.
- `target/step-12-error-logs/results.json`: 26 bài lỗi giả lập/FFmpeg thật.
- Lệnh chạy lại trong [DEVELOPMENT_COMMANDS](../DEVELOPMENT_COMMANDS.md#8-download-dicomjpegmp4-step-12).

Store thật có WADO-URI trả body JPEG với MIME image/png; Rust dùng WADO-RS
rendered có MIME image/jpeg. Không tính WADO-URI media contract là đạt.

## Streaming và lỗi đã chốt

Rust chuẩn bị xong output trước khi gửi headers, sau đó stream file theo chunk
64 KiB với Content-Length. Thời điểm first byte/lỗi khác Java servlet streaming.

- Trước headers: nguồn non-200 hoặc body cắt trả JSON 500; deadline 504,
  quá tải 503, vượt tổng quota storage 413. Không gửi ZIP dở hoặc body lỗi store.
- Sau headers: status đã commit là 200; store lỗi không ảnh hưởng file đã chuẩn bị.
  Nếu file output bị cắt, client nhận biết thiếu byte qua Content-Length.
- Client ngắt khi encode: FFmpeg tiếp tục tới completion rồi mới dọn tài nguyên.
  Deadline khi native còn chạy trả 504; readiness tạm 503, health 200 và phục hồi
  khi native kết thúc. Không tạo JVM thay thế.

Các hành vi lỗi trên đã test có kiểm soát bằng nguồn giả lập; xác nhận với nguồn
thật là mục tiếp theo, không được đánh dấu đạt từ kết quả giả lập.
