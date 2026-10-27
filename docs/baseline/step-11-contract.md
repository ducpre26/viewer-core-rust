# Baseline Step 11 — UC-004

Ngày 2026-10-27; Java revision `2373896fca106bab446c0e14bb38f0b8d1725949`.
Nguồn: WadoRsAPI, WadoRsServiceImpl, WadoRsProxy, WadoUriProxy,
SessionServiceImpl, WebRequestFilter và WadoRsAPITest.

## Phạm vi người dùng đã chốt

- Bỏ nhánh key image cục bộ ở Step 11: không đọc STUDY_KEY_IMAGE, ghép metadata,
  bổ sung series hoặc trả frame từ file cục bộ. Các dữ liệu này chỉ lấy upstream.
- Giữ hành vi Java cho study ngoài session, kể cả HTTP 500 do scoped lookup
  trả null. Không tự đổi thành 403.
- Chưa prefetch metadata sau tạo session; quay lại kiểm thử Step 8 sau.
- Proxy thuần không parse/normalize DICOM JSON và không cần JNI, codec hoặc
  migration mới. Không thay đổi snapshot Java hoặc schema SESSION.

## HTTP contract

- Prefix `/ws/rest/v1/session/{sessionID}`. WADO-RS dưới
  `/wado-rs/studies/{studyUID}`: study, thumbnail, metadata, series list,
  series, series thumbnail/metadata, instance list, instance, instance
  thumbnail/metadata, frames, frame thumbnail, `{bulk}/{bulkData}`.
  Có tổng cộng 14 route WADO-RS và một route `/wado-uri`.
- Session lấy từ cache rồi DB theo nền tảng đã có. Session thiếu/hết hạn hoặc
  share-key sai bị filter trả 401. Session hợp lệ nhưng study ngoài tập study:
  500 body rỗng, không gọi store. Không bổ sung permission guard.
- Store URL/auth chỉ lấy từ session; không forward Authorization của caller.
  WADO-RS nối đường dẫn studies/... vào store URL. WADO-URI áp dụng Java
  `storeUrl.replace("wado-rs", "")`, nối `/wado`.
- WADO-URI bắt buộc query studyUID, seriesUID, objectUID; requestType mặc định
  WADO khi thiếu/rỗng; contentType thiếu được bỏ khỏi query (annotation default
  trên Feign không tự áp dụng khi controller truyền null). Unknown query bị bỏ.
  Thiếu query bắt buộc trả 500 INTERNAL_ERROR qua advice với tên field thiếu.
- Thumbnail chuyển toàn bộ query, duplicate key lấy giá trị đầu. Route khác bỏ
  query ngoài các field WADO-URI. Accept chỉ forward ở instance, frames và frame
  thumbnail; mặc định `*/*`. Transfer syntax là một phần của Accept/media type.
- Feign method trả `Response`, nên non-2xx không qua ErrorDecoder: giữ status
  và body upstream, bao gồm 401/403/404/429/500/503. Mất kết nối/timeout trước
  headers trả 500 body rỗng. Lỗi sau headers chấm dứt stream, không tạo JSON lỗi
  hoặc đổi status đã gửi.
- Controller chỉ chuyển Content-Type. Không chuyển Content-Length, Set-Cookie,
  upstream CORS/security header hoặc Content-Disposition. Spring tự thêm
  `Content-Disposition: inline;filename=f.txt` trên đường dẫn có đuôi UID số;
  Rust giữ trường hợp này bằng header do ứng dụng tạo.
- Khi upstream thiếu Content-Type, fixture Spring chọn application/json cho
  Accept `*/*`, image/jpeg cho image/jpeg và giữ multipart/related với tham số
  transfer-syntax (bỏ whitespace sau dấu chấm phẩy).
- Streaming giữ nguyên byte, không gom toàn bộ payload; không decompress,
  normalize hay transcode. Metadata sai JSON vẫn được truyền nguyên byte.
  HTTP client giữ connect 10s/read 60s, follow redirects như nền tảng QIDO;
  không tự lấy proxy từ environment. Đọc chậm có backpressure; thả downstream
  body sẽ thả upstream stream, không có producer task/channel riêng.

## Bằng chứng và giới hạn

`tools/Step11Baseline.java` chạy controller/service/Feign SpringMvcContract thật,
mock transport store và session collaborator; key image trả null. Không sửa
nguồn Java. 31 fixture tại `step-11-java.json`, timestamp lỗi phát sinh được bỏ
để so sánh. Rust đối chiếu 30 fixture phù hợp controller; case missing-session
được kiểm tra qua filter thật với 401 thay vì 500 của controller standalone.
URL query được so sánh theo key/value; frame list `1,2` và `1%2C2` tương đương.

Postman mock lấy từ contract và fixture Java này; CLI chưa đăng nhập nên không
tra cứu mock của bên sở hữu store. Chỉ chạy cục bộ, không dùng credential hoặc
PHI thật. Test TCP riêng kiểm tra transport chậm/truncated/timeout mà mock nhỏ
không mô phỏng. Không khẳng định đã kiểm tra PACS thật, frontend hiển thị ảnh,
release build hoặc CI Windows/Ubuntu trong lượt này.
