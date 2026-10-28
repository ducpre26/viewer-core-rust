# Contract tải dữ liệu — Step 12

**Khác Java:** Rust không dùng Orthanc transcode/native; DICOM giữ nguyên byte,
JPEG lấy từ WADO-RS rendered của store. MP4 giữ JavaCV/FFmpeg theo T14.

Revision Java: `2373896fca106bab446c0e14bb38f0b8d1725949`.
Revision Rust khi bắt đầu: `a5dfaedffcf4d0f1c6c073376690177bc9e2539d`.

`tools/Step12Baseline.java` biên dịch nguyên trạng mã nguồn DownloadAPI,
DownloadServiceImpl, DownloadType và TranscodeService vào thư mục target riêng,
sử dụng classpath của bản build Java. Không sửa mã nguồn Spring Java hoặc các
constructor hiện có. `tools/baseline_download.py` xuất kết quả ra `step-12-java.json`.

Bộ 24 trường hợp bao phủ kiểu tải mặc định/không được hỗ trợ ở mọi phạm vi;
header và các mục trong ZIP khi tải DICOM/JPEG theo instance, series và study;
JPEG một/nhiều frame; series rỗng khi tải DICOM/JPEG/MP4; và study ngoài phạm vi
phiên. Các trường hợp kiểu tải không được hỗ trợ chạy qua MockMvc với bộ xử lý
ngoại lệ thật (415 / UNSUPPORTED_MEDIA_TYPE). Các trường hợp thành công và kiểm tra
output gọi trực tiếp StreamingResponseBody thật với MockHttpServletResponse;
chưa phải kiểm thử xuyên suốt qua servlet container và PACS thật.

Lớp truyền dữ liệu, dịch vụ session/config và thao tác transcode DICOM được thay
bằng thành phần giả lập phục vụ kiểm thử. Một lớp render riêng trả byte đánh dấu theo từng frame. Các byte này chỉ dùng để kiểm tra cách đóng gói và đặt tên, không phải
JPEG hoặc DICOM hợp lệ. Kiểm thử HTTP Rust dùng mã xử lý metadata thật của facade,
nhưng thay thân các method codec trong JAR dữ liệu kiểm thử chỉ nằm dưới target.
Artifact production không chứa các codec giả lập này.

Các kết quả chính:

- Study và instance chấp nhận DICOM/JPEG; series chấp nhận thêm MP4. Việc diễn giải
  kiểu tải không phân biệt hoa thường và không loại bỏ khoảng trắng đầu/cuối.
  Giá trị thiếu hoặc không nhận diện được chuyển thành OCTET_STREAM rồi bị từ chối.
  Controller không có annotation kiểm tra quyền bổ sung.
- Khi study ngoài phạm vi tải của phiên, dịch vụ trả về stream null, nên response
  sau xác thực là 200 với body rỗng. Hành vi này khác WADO trả 500 rỗng. Phiên
  không tồn tại/hết hạn hoặc share-key sai vẫn bị filter trả 401; kiểm thử Rust
  bao phủ các trường hợp này qua lớp xác thực hiện có.
- Tên file khi tải series dùng UID của study, bao gồm `{studyUID}.mp4`.
- JPEG một frame của instance dùng `application/jpeg`; JPEG nhiều frame được đóng
  ZIP với thư mục `1/` và các mục `1/{frame}.jpeg`, chỉ số frame bắt đầu từ 0.
- Đường dẫn trong ZIP dùng AttributesFormat thật của dcm4che và các giá trị mặc
  định của Java. Khi thiếu số thứ tự series, giá trị thay thế là `1` khi tải
  series và `0` cho series đầu tiên trong study. HashMap không bảo đảm thứ tự
  series ổn định trong study. Đối chiếu tên và nội dung các mục ZIP, không so
  toàn bộ byte nén hoặc timestamp.
- Body/ZIP entry DICOM bằng toàn bộ byte nguồn, giữ nguyên transfer syntax.
  JPEG lấy qua
  `/studies/{study}/series/{series}/instances/{sop}/frames/{frame}/rendered`
  với Accept image/jpeg, quality=90; chỉ số frame gửi store bắt đầu từ 1, tên file
  ZIP vẫn bắt đầu từ 0. Byte JPEG bằng output store, không buộc giống renderer
  local Java gốc. MP4 giữ JavaCV
  H.264/YUV420P, FPS theo cấu hình (mặc định Java là 1), bitrate 400000 và các
  tùy chọn recorder gốc.

Các giới hạn của bản triển khai:

- Output được chuẩn bị trong thư mục job có giới hạn dung lượng trước khi luồng
  HTTP đọc file theo nhu cầu phía nhận, mỗi chunk 64 KiB. ZIP lưu dữ liệu không
  nén, trong khi Java mặc định dùng deflate. Tên và nội dung các mục khớp dữ liệu
  kiểm thử, nhưng cách nén, thời điểm gửi byte đầu tiên và thời điểm báo lỗi chưa
  giống hoàn toàn servlet streaming. Lỗi codec/nguồn dữ liệu trước khi Rust trả
  response được chuyển thành 500 với thông tin lỗi đã lược bỏ chi tiết nhạy cảm;
  quá tải trả 503, quá deadline trả 504, vượt tổng hạn mức lưu trữ trả 413. Chưa
  kiểm chứng đầy đủ tính tương đương với servlet khi lỗi xảy ra sau khi response
  đã được gửi; các kết quả trên chưa chứng minh toàn bộ hành vi khớp baseline.
- Từ chối đường dẫn ZIP không an toàn và tên mục trùng lặp. Rust tự sinh tên file
  trên hệ thống file; không dùng metadata bệnh nhân làm đường dẫn trên đĩa.
- Job giữ khóa thư mục và suất xử lý đã cấp đến khi Java hoặc thao tác I/O chặn
  hoàn tất, kể cả khi request bị hủy hoặc hết deadline. Tác vụ dọn file theo TTL
  bỏ qua thư mục đang khóa và chỉ xóa thư mục job thuộc ứng dụng đã quá hạn bên
  trong thư mục gốc được cấu hình. Khi gửi xong body hoặc body bị hủy, thư mục
  được dọn theo vòng đời tài nguyên sau khi không còn thao tác sử dụng.
- DICOM/JPEG instance không cần JVM. JPEG instance đọc số
  frame từ DICOM JSON bằng Rust; study/series dùng Java parse metadata/tạo path.
  File DICOM chịu hạn mức byte/storage, không chịu giới hạn pixel/frame của codec.
  JPEG chịu giới hạn byte, frame và kích thước pixel từng frame; Rust kiểm tra
  MIME, SOI/EOI và SOF trước khi trả file. Store phải hỗ trợ rendered-frame JPEG;
  không có fallback native hoặc đổi ảnh PNG thành nhãn JPEG.
- Khi bật tính năng, phải khai báo cấu hình có kiểu dữ liệu rõ ràng trong
  `[download]`. Tính năng mặc định tắt nếu không khai báo.
  `config/download.example.toml` chứa giới hạn dành cho dữ liệu thử nghiệm,
  không phải quyết định P1–P4 cho production. Không thêm bảng hoặc migration DB.

Kiểm chứng native được thực hiện riêng: `DownloadSmoke` tạo DICOM giả lập gồm hai
frame 16×16, kiểm tra mapping metadata, giá trị thay thế và giới hạn; encode MP4
bằng Java binding của production rồi giải mã bằng JavaCV. Tùy chọn `--independent`
giải mã thêm bằng FFmpeg executable độc lập. Chưa có kết quả đạt
cho toàn bộ bộ dữ liệu transfer syntax, tình huống đĩa đầy, hoặc kiểm tra bộ nhớ và
chạy kéo dài với series lớn.

`step-12-media.json` lưu DICOM đầu vào giả lập và MP4 thật do method
`convertJpegToMp4` của dịch vụ Java nguyên trạng tạo ra từ các frame đọc bằng
dcm4che ImageIO. Bộ giải mã FFmpeg độc lập xác nhận output của dịch vụ gốc,
facade được gọi trực tiếp và Rust/JNI đều cho cùng 512 byte grayscale sau giải mã
(hai frame 16×16, FPS 1). Đây là một trường hợp MP4 chuẩn để đối chiếu với dữ liệu
nhỏ, chưa chứng minh mọi đầu vào ảnh màu, ảnh nén hoặc video đều tương đương.

Kiểm chứng với store thật trong STORE.md: study có 1 series,
2 instance. DICOM instance qua Rust với JNI tắt và DICOM trong ZIP study/series với
Java metadata thật đều bằng byte nguồn; transfer syntax JPEG Baseline
`1.2.840.10008.1.2.4.50` được giữ nguyên. pydicom đọc file, Pillow giải mã frame
encapsulated JPEG và JPEG do server render, kích thước lần lượt 1795×563 và 1440×1080.
Kết quả tại `target/step-12-logs/live-store/result.json`, metadata/media thật chỉ
lưu dưới target theo cho phép của người dùng; không đưa vào fixture Git.

Store có sai MIME ở WADO-URI: request JPEG trả body JPEG nhưng header `image/png`.
WADO-RS `frames/1/rendered` trả đúng `image/jpeg`; ảnh được Pillow giải mã thành công.
Không coi WADO-URI media contract là đã pass. Rust dùng WADO-RS rendered cho mọi frame JPEG và xác nhận byte qua HTTP thật.
Lệnh chạy lại: `tools/verify_live_download.py`, chỉ GET tới Orthanc; Postman CLI
chạy 3 assertion với `--no-report-events`. Session test chỉ ở cache ứng dụng Rust.

Cả hai JPEG instance qua Rust khi JNI tắt
được Pillow giải mã và bằng byte JPEG rendered từ store (quality=90). ZIP study
và series cho cả DICOM lẫn JPEG đều có đúng 2 file, byte từng file khớp nguồn.
`result.json` ghi `localOrthancDependency=false` và `rustRemoteJpegVerified=true`.
HTTP synthetic fixture `step-12-rendered-jpeg.json` dùng JPEG hợp lệ, phân biệt từng
frame; test giữ các tên ZIP cũ, từ chối sai MIME/status, dữ liệu giả JPEG và ảnh
bị cắt cụt. Fixture Java cũ chỉ còn là baseline route/header/naming; byte DICOM
và JPEG expected theo hai quyết định mới của người dùng.

## Thời điểm streaming và báo lỗi — kiểm chứng 2026-10-28

Chốt hành vi Rust: tải toàn bộ nguồn, hoàn tất ZIP/MP4 và mở file output trước
khi gửi HTTP headers. Sau đó đọc file theo chunk 64 KiB, có `Content-Length`
bằng kích thước file đã chuẩn bị. Đây là khác biệt có chủ đích với Java servlet
ghi output trong quá trình xử lý; không tuyên bố tương đương thời điểm first byte
hoặc lỗi sau commit. Không sửa Java gốc trong lượt này.

| Thời điểm / sự cố | Hành vi đã kiểm chứng trên Rust |
|---|---|
| Store mới gửi một phần body, chưa EOF | Chưa có headers/body tải xuống; file tạm nội bộ đã nhận byte |
| Store trả 401/403/404/429/5xx, metadata lỗi hoặc body bị cắt | Trả JSON lỗi 500 trước headers tải file, không có Content-Disposition, không lộ body lỗi store |
| Store chậm headers/body vượt deadline | JSON 504; kết nối không tới được store có thể là 500 khi lỗi kết nối đến trước deadline hoặc 504 khi deadline đến trước |
| Instance thứ hai/frame thứ hai lỗi | Cả request lỗi trước response; không trả ZIP dở |
| Store lỗi sau khi Rust đã trả headers | File đã chuẩn bị vẫn tải đầy đủ, đúng SHA-256 nguồn; request mới trả lỗi 500 |
| File tạm bị cắt sau headers (fault injection cục bộ) | Status đã commit là 200; không thể đổi sang JSON 500. Content-Length làm client nhận biết thiếu byte qua IncompleteRead |
| Client ngắt trong lúc gửi file | Body bị hủy, file/permit được giải phóng; request kế tiếp hoạt động |
| Client ngắt khi FFmpeg thật đang encode | Native tiếp tục tới completion; file/permit còn được giữ, request mới nhận 503 với max_jobs=1; TTL bỏ qua job active |
| Deadline trong lúc FFmpeg thật encode | HTTP 504; job còn giữ tài nguyên, ready=503 và health=200; completion dọn file và ready trở lại 200, không thay JVM |

Fixture synthetic: 384 frame grayscale 512×512 nhiễu, DICOM 100663810 byte;
MP4 thật giải mã bằng FFmpeg executable độc lập đủ 384 frame. Đây là bài kiểm
tra vòng đời/lỗi, không phải chứng nhận hiệu năng, memory peak hay corpus codec.
Runner: `tools/test_download_failures.py`; bằng chứng cục bộ tại
`target/step-12-error-logs/results.json`. Store lỗi là mock localhost, không gây
sự cố trên PACS thật. Bổ sung kiểm tra HTTP status chung để sửa lỗi phát hiện:
trước sửa, store 401 có thể bị trả thành DICOM 200 chứa JSON lỗi.
