# Step 13 - UC-006 Tiếp nhận và xuất key image

## Tính năng

Chuyển luồng tiếp nhận key image, lưu pending theo study, retry và gửi STOW sang Rust theo hành vi Java. Không tái thiết kế nghiệp vụ từ mô tả UC-006.

**JNI:** Tạo/validate/ghi DICOM.

## Nguồn Java

- [module/stowrs/rest/StowRSAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/rest/StowRSAPI.java)
- [module/stowrs/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/service/)
- [module/keyimage/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/keyimage/)
- [UC-006](../../../1pacs-viewer-core/docs/use_cases/UC-006-tiep-nhan-va-xuat-key-image.md)

## Contract Java làm chuẩn

Theo xác nhận người dùng ngày 2026-11-05, code Java là chuẩn của Step 13;
UC-006 và checklist cũ có thể lệch, không dùng chúng để tự thay đổi hành vi.
Các chi tiết dưới đây được đối chiếu tĩnh tại Java revision
`2373896fca106bab446c0e14bb38f0b8d1725949`; Rust baseline
`74a7b414ab0193bca3df9b48f8026f724991f2e8`. Đã thu fixture service Java synthetic tại `docs/baseline/step-13-java.json` và
kiểm chứng phạm vi bên dưới; chưa có Spring container/DB Java thật. Kết quả đọc
source không thay thế kiểm thử thực tế hoặc mở rộng bằng chứng ra ngoài corpus.

### Tiếp nhận và thứ tự tạo

- `POST /ws/rest/v1/session/{sessionID}/studies` nhận `multipart/related`
  có boundary. Phần DICOM JSON được parse thành instances; bulk data được spool
  và nối theo `Content-Location`/`BulkDataURI`.
- Giữ filter session/expiry/share-key dùng chung. Controller không có permission
  STOW riêng; validation gọi `getSession(sessionID)`, không kiểm tra study scope.
  Không tự thêm điều kiện một study/request hoặc quyền mới.
- Sau parse, `ValidationServiceImpl.validateBeforeSaveDicomFile` kiểm tra danh
  sách không rỗng, StudyInstanceUID của instance đầu tiên và giới hạn số ảnh.
  Record được tìm bằng StudyInstanceUID đầu tiên. SeriesInstanceUID lấy từ
  instance đầu tiên; nếu null thì dùng series pending cũ hoặc sinh UID mới.
  Series đã chọn được áp lên toàn bộ instances. Không tự từ chối series khác
  hoặc sửa hành vi batch nhiều study khi chưa có thay đổi nghiệp vụ được duyệt.
- **Tạo/cập nhật pending trước khi ghi DICOM.** Record mới lấy URL/auth từ session,
  `succeed=false`, retry=1. Record cũ tăng số ảnh, đặt `succeed=false`, retry=1,
  description=`Send with new key image`; không cập nhật URL/auth hoặc series
  của record cũ trong nhánh này.
- Sau đó mới lần lượt resolve bulk reference, bổ sung thuộc tính và ghi từng
  DICOM trong `StowRSServiceImpl.storeDicomObject`. `supplementAttrs` chỉ chạy
  khi resolve được bulk có media type; không nâng các kiểm tra trong nhánh này
  thành validation bắt buộc cho mọi đầu vào. Giữ các nhánh pixel/document và
  transfer syntax thực tế của Java; không mặc định mọi input đều là JPEG.
- Cuối cùng bổ sung retrieve/bulk URI, ghi/ghép `metadata.json`, trả 200 với mảng
  DICOM JSON. Đây là thành công tiếp nhận local, chưa phải kho đã nhận.
- `finally` dọn thư mục spool của request cả khi thành công và thất bại.
  Ingest lỗi sau khi cập nhật DB/ghi một số file **không rollback pending hoặc
  xóa toàn bộ file series** trong Java. Không tự thêm atomic ingest/rollback;
  fixture phải ghi nhận DB/file còn lại tại từng điểm lỗi.

Call site: [store](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/service/StowRSServiceImpl.java#L111),
[validation/pending](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/service/ValidationServiceImpl.java#L42),
[ghi DICOM và metadata](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/keyimage/service/FileStorageServiceImpl.java#L45).

### Pending, cấu hình, xuất và cleanup

- `STUDY_KEY_IMAGE.id` là StudyInstanceUID; một record pending cho study,
  không tạo một record mới cho mỗi request. Mapping schema theo
  [db/init.sql](../../../1pacs-viewer-core/db/init.sql#L58), gồm audit/defaults.
  Tái sử dụng SESSION hiện có; Java UC-006 không đọc CONFIG để lấy các setting
  key image. Cấu hình vận hành Rust vẫn qua typed TOML/Summer hiện có.
- Giới hạn Java mặc định 10 tính trên số ảnh pending cộng số mới; không truy vấn
  số ảnh đã gửi trong kho. Record bị xóa sau gửi thành công nên bộ đếm này
  không phải hạn mức tổng số key image suốt vòng đời study.
- Scheduler chỉ chạy khi `sendToStore` bật (Java mặc định false), fixed delay
  mặc định 30000 ms. Chọn trang đầu tối đa 100 record `succeed=false` và
  `num_of_retry<=3`. Ingest đặt retry=1; lần gửi thất bại tăng một, ghi reason;
  retry=4 không được chọn tiếp và không tự xóa record/file đã hết lượt.
- Gửi POST tới `store_url + "/studies"`, chuyển tiếp `store_auth`, multipart
  application/dicom. `FileStorageServiceImpl.writeKeyImage` duyệt mọi regular
  file trong thư mục series, gồm `metadata.json`; lỗi đọc file được log rồi
  tiếp tục. Không tự đổi thành chỉ gửi `.dcm` hoặc fail-fast.
- `stowRs` coi HTTP 2xx là thành công, không phân tích response body theo
  instance. Giữ cách này cả với body báo partial failure; không thêm retry
  theo instance hoặc điều kiện xác nhận tất cả SOP trước cleanup.
- Thành công: xóa record rồi purge thư mục series. Thất bại/exception: scheduler
  tăng retry và ghi description. Giữ thứ tự và kiểm chứng cả lỗi khi cleanup;
  không mô tả DB delete và filesystem purge là một transaction nguyên tử.
- Java còn ghép series/metadata và đọc frame local khi pending qua
  `WadoRsServiceImpl.getSeries`, `getSeriesMetadata`, `getInstanceFrames`.
  Đối chiếu/port các nhánh liên quan tại Step 13; Step 11 đã để ngoài phạm vi
  các nhánh local này, nên proxy Rust hiện tại chưa đáp ứng phần đó.

Call site: [scheduler](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/keyimage/job/KeyImageScheduler.java#L42),
[STOW](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/service/StowRSServiceImpl.java#L435),
[ghi multipart](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/keyimage/service/FileStorageServiceImpl.java#L157),
[đọc local](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/service/WadoRsServiceImpl.java#L94).

### Ranh giới facade và bằng chứng cần thu

Rust quản lý API/session, pending DB, scheduler và STOW; Java facade xử lý
DICOM theo các nhánh nguồn. Tên `create-key-image` chỉ là định hướng, chưa phải
signature hay một lời gọi tạo trọn batch đã chốt. Thiết kế facade phải cho phép
parse/đọc metadata cần cho validation trước cập nhật pending, rồi bổ sung/ghi
DICOM sau cập nhật pending; không chuyển toàn bộ validation nghiệp vụ/DB vào Java.

Giữ yêu cầu an toàn JNI mục 5.7: worker sở hữu buffer/file lease đến khi Java
thực sự kết thúc; request timeout/disconnect không được xóa file Java đang dùng.
Điều này không cho phép tự thay đổi cleanup nghiệp vụ của Java. Fixture synthetic
cần ghi HTTP/header/body, thuộc tính DICOM, DB và cây file trước/sau; chuẩn hóa
UID/thời gian sinh tự động. Các trường hợp source chưa đủ xác định phải thu
runtime trước khi kết luận, đặc biệt content type response, body 401, partial
write, batch nhiều study/series và lỗi cleanup. Không sửa source/snapshot Java.

## Việc cần làm

- [x] Thiết kế facade parse/bổ sung/validate/ghi DICOM theo thứ tự Java nêu trên; không coi create-key-image là contract tạo nguyên lô; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [x] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [x] Mapping STUDY_KEY_IMAGE và migration/repository SeaORM theo DDL Java; tái sử dụng SESSION, lưu file key image và typed TOML settings; không thêm CONFIG setting mà Java không dùng.
- [x] Parse multipart và validate đúng nhánh Java; không tự thêm study scope, đồng nhất study hoặc required attributes ngoài baseline.
- [x] Tạo/cập nhật pending theo study trước khi ghi DICOM, giữ số ảnh, retry=1 và description theo nhánh Java.
- [x] Java facade resolve bulk, bổ sung/validate thuộc tính và ghi lần lượt DICOM theo source; sau đó ghi metadata/response URI.
- [x] Scheduler lấy tối đa 100 batch, retry `<= 3`, gửi STOW khi feature bật.
- [x] Giữ payload STOW và tiêu chí HTTP 2xx của Java, kể cả metadata.json và lỗi đọc file chỉ log; kiểm chứng bằng fixture.
- [x] Cleanup đúng phạm vi/thứ tự Java: ingest dọn request spool; STOW thành công xóa record rồi thư mục series; không rollback pending/file đã ghi khi ingest lỗi.
- [x] Port các nhánh WADO series/metadata/frame local liên quan key image đang pending theo Java.
- [x] Áp dụng [mục 5.7](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership) cho JNI create/validate DICOM; cleanup không xóa file Java còn sử dụng.

## Kiểm tra hoàn thành

- [x] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [x] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [x] Dùng UC-006 main flow, A1..A3, BR-001..BR-004 làm chỉ mục coverage, expected result theo contract Java ở trên khi khác UC: retry 1/3/4, 100/101 record, feature tắt, hạn mức pending, batch nhiều study/series, lỗi giữa lô, STOW 2xx có partial failure, lỗi đọc file và cleanup; kiểm tra đọc local trước khi xuất.
- [x] Cancellation khi facade đang dùng file: không cleanup sớm; record/temp state vẫn được kiểm chứng theo contract.
- [x] Đạt khi: output/response và pending/retry/cleanup state khớp baseline Java, bao gồm nhánh lỗi; không dùng hậu điều kiện UC mâu thuẫn source để đổi hành vi. Kiểm chứng DICOM của luồng thành công bằng corpus tính năng.

## Bằng chứng triển khai và giới hạn

- `tools/baseline_key_image.py`: compile service ingest/validation Java gốc tại
  revision đã ghi, repository/session double, file thực. 10 case: thành công,
  thiếu boundary, rỗng, thiếu study, vượt giới hạn, lỗi giữa lô, nhiều study,
  JPEG, pixel thô và thiếu thuộc tính pixel. Fixture lưu request bytes, response,
  pending/file state và DICOM synthetic. Chưa chứng minh Spring container/DB
  Java thật; body/header auth dùng foundation đã có.
- `tools/smoke_key_image.py`: test HTTP router với JVM/JAR production và
  PostgreSQL thật, Postman STOW cục bộ; kiểm tra 10 fixture, share-key,
  cập nhật count/reset retry, đọc series/metadata/frame local, feature tắt,
  retry 1/3/4, STOW 202 có body failed-instance, payload gồm metadata.json,
  record/file cleanup và một lượt gửi đúng 100 trong 101 record đủ điều kiện.
  Cần DB test riêng, bảng pending rỗng trước khi chạy; không dispatch record có sẵn.
- pydicom so toàn bộ dataset và transfer syntax với DICOM Java gốc sau chuẩn hóa
  study ID synthetic; Pillow giải mã JPEG encapsulated 8×8, kiểm tra raw pixel
  8×8/64 byte. Kết quả `target/step-13-logs/independent-output.json`.
- Binary thật qua TCP: ingest JPEG, URL response, WADO local, pending tồn tại sau
  restart, scheduler xuất/cleanup khi bật lại và shutdown sạch. Kết quả
  `target/step-13-logs/tcp-result.json`.
- `tools/smoke_jni.py`: 25 subprocess case đạt, gồm `key-image` kiểm tra call đã
  vào Java trước timeout, disconnect không thả owner sớm, bounded queue,
  deadline trong queue không gọi Java, 100 exception call và probe sau đó.
  Fixture điều khiển thời gian là test-only; corpus DICOM dùng JAR production.
- `cargo test --workspace --locked`: 26 đạt, 13 ignored; các test ignored chỉ được
  tính khi chạy riêng bằng runner tương ứng. PostgreSQL regression filter
  `postgres_migrations_transaction_readiness_and_close`: 9 đạt.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --locked`, facade build/packaging và
  `python tools/test_facade_metadata.py` (3 test) đạt. `python tools/smoke_app.py`
  đạt dev/prod, cấu hình Summer, startup failure và graceful shutdown.
- Resource adaptation: facade đóng metadata reader/writer (Java nguồn chỉ flush,
  làm Windows giữ handle), giữ file containment và không log UID/path/codec output.
  Không đổi thứ tự pending trước DICOM hoặc rollback phần đã ghi. Fixture tạo sẵn
  thư mục series để tránh lỗi `File.setWritable` của Java nguồn trên ACL Windows.
- Chưa xác nhận Linux CI, PACS thật hoặc toàn bộ định dạng codec/document được
  Java chấp nhận; helper giữ các nhánh nguồn, corpus hiện kiểm chứng JPEG/raw.
  Nhánh QuickTime của nguồn cần executable `ffmpeg`. Không coi codec khác,
  production sizing/soak hoặc các kiểm chứng Step 12 đã hoãn là đạt.

## Tiến độ

- Trạng thái: `in_review` — implementation/local đã đạt và từng đạt QA với study synthetic riêng; kiểm chứng trên study gốc ở archive department02 hiện chưa đạt readback. Giữ nguyên contract Java; chưa commit.
- Đã làm: API ingest, entity/migration/repository, typed TOML settings, facade parse/write/merge/frame, owned JNI job, scheduler STOW/retry/cleanup và WADO local theo Java. Lệnh tái lập tại mục 9 của DEVELOPMENT_COMMANDS.md.
- Tiếp theo: Xác minh lưu/index/readback key image trên study gốc department02 bằng bằng chứng phía store hoặc API truy xuất; sau đó chạy lại luồng qua Rust với đúng URL/auth/payload nguồn. Người dùng đã yêu cầu commit và push phần Step 13 hiện có ngày 2026-11-06 dù kiểm chứng này còn mở; giữ `in_review`, không coi commit là xác nhận readback đã đạt. Không deploy hoặc chuyển step. Giữ contract Java đã chốt.
- Vướng mắc: Gửi trực tiếp file enriched vào department02 trả HTTP 200 nhưng WADO readback chưa đạt và QIDO series mới rỗng. Hiện tượng tái hiện khi bỏ qua BE Rust; chưa đủ bằng chứng quy lỗi cho Rust, file DICOM hoặc store. Kết quả thành công ở archive/study synthetic trước không thay kiểm chứng study gốc hiện tại. Các giới hạn Spring container/DB Java, Linux và codec/PACS production vẫn chưa được kiểm chứng.
- Kiểm tra gần nhất: Các lệnh/kết quả trong mục bằng chứng; log runtime ở `target/step-13-logs/`, JNI ở `target/step-03-logs/`. Không tính ignored là pass.
- Cập nhật: 2026-11-06 — Codex; commit/push theo yêu cầu trực tiếp của người dùng. Người dùng yêu cầu tạo lại commit để gồm thay đổi .gitignore và docs/STORE.md, docs/STORE_2.md với thông tin store/auth hiện có. Không đưa target, dữ liệu bệnh nhân hoặc các bộ Postman ngoài phạm vi vào commit.

## Kiểm chứng QA thực — 2026-11-06

Theo yêu cầu người dùng, đọc URL/auth từ `docs/STORE.md`; chỉ GET study tham
chiếu, tạo hai study synthetic UID riêng (JPEG Baseline và raw pixel Explicit
VR Little Endian). Không sửa study tham chiếu. Hai study synthetic giữ trên QA
để review; UID trong `target/step-13-logs/live-qa/result.json`.

- Build binary debug hiện tại đạt. Runner `tools/verify_live_key_image.py` dùng
  facade production/JVM thật, HTTP localhost và PostgreSQL database mới riêng;
  database thử nghiệm đã được xóa, config có credentials đã được dọn.
- Ingest 200, pending 1:false và metadata local đạt cho cả hai. JPEG local frame
  200; raw local frame 404 do helper Java chỉ hỗ trợ encapsulated compressed
  pixels. Đối chiếu source Java `FileStorageServiceImpl.getInstanceFrame` xác
  nhận cùng giới hạn; không coi raw local frame là đạt.
- Sau restart bật scheduler, QA có cả hai DICOM; QIDO/metadata và WADO DICOM thật
  trả 200. pydicom kiểm tra UID, transfer syntax và PixelData khớp local;
  Pillow giải mã JPEG 8×8; raw có 8×8/64 byte.
- **End-to-end thất bại:** còn 2 pending, retry=4, file local không được cleanup.
  Runner exit 1, `passed:false`; không lấy readback thành công làm pass toàn flow.
- Thử đối chứng trên đúng hai study synthetic đã tạo: multipart gồm DICOM và
  metadata.json trả **400 (2 parts)** cho cả hai; chỉ DICOM trả **200 (1 part)**
  cho cả hai. Không sửa production để tạo đối chứng. Source Java `writeKeyImage`
  cũng duyệt mọi regular file và gắn Content-Type application/dicom.
- Postman CLI truy vấn metadata QA: 2 assertions mỗi case, tổng 4 đạt, cả hai
  lần exit 0, `--no-report-events`; env credential tạm đã được dọn.

Bằng chứng: `target/step-13-logs/live-qa/result.json`, `diagnosis.json`,
`{jpeg,raw-pixel}-stow-{all,dicom-only}.json`, `*-postman.log`,
`*-local.dcm`, `*-qa.dcm` và `app-{False,True}.log`. Không lưu credentials hoặc
metadata study tham chiếu vào tài liệu version control. Đây là kiểm chứng
store QA thật từ binary Windows local; chưa thay staging Linux/Docker/soak.

### Store mới từ STORE_2.md — 2026-11-06

Giữ nguyên contract Java, chưa sửa payload STOW. Build debug đạt; chạy runner với
`--store-file docs/STORE_2.md --output target/step-13-logs/live-qa-store-2`.
Preflight GET series study tham chiếu, có Basic Auth và Accept
application/dicom+json, trả HTTP 200 nhưng Content-Type text/html, 1192 byte,
là trang SPAX 3.4.0-store. GET base và studies?limit=1 cũng trả cùng trang HTML.
Không thể dùng HTTP 200 này để xác nhận QIDO hoặc credentials hợp lệ.

GET thử cùng host: /ws/rest/wado-rs và /ws/rest/v1/wado-rs trả 404;
/dicom-web trả cùng HTML; /api/wado-rs trả 401 với Basic Auth đã cung cấp.
Các tiền tố thử không được coi là endpoint được xác nhận và không sửa STORE_2.md.

Kết quả: chưa chạy được ingest/STOW/readback trên store mới; không có remote
write và chưa tạo database test. Runner exit 1 tại reference-QIDO-preflight.
Cần URL DICOMweb thực tế cùng cơ chế xác thực tương ứng để tiếp tục. Bằng chứng:
`target/step-13-logs/live-qa-store-2/result.json`, `endpoint-probes.json` và
`probe-{base,series,studies}.body`. Không kết luận store mới có cùng lỗi store cũ.

### Chạy lại STORE_2 sau khi sửa URL — 2026-11-06

URL mới có tiền tố /api/wado-rs. Runner dừng tại QIDO preflight với HTTP 401,
body rỗng. GET base, series của study tham chiếu và studies?limit=1 đều trả
401 với Basic Auth từ STORE_2.md; không có redirect hoặc WWW-Authenticate.
Chưa xác định credentials sai hay endpoint cần cơ chế xác thực khác.
Chưa gửi STOW, chưa tạo database test hoặc ghi dữ liệu lên store.
Giữ nguyên contract Java/Rust. Cần xác nhận auth của endpoint để tiếp tục.
Bằng chứng: `target/step-13-logs/live-qa-store-2-retry/result.json` và
`auth-probes.json`. Runner exit 1; chưa kiểm chứng STOW/readback/cleanup.

### STORE_2 lần cập nhật cuối — 2026-11-06: đạt luồng QA

Thông tin mới dùng endpoint external archive và Bearer auth. Runner đã hỗ trợ
header Basic/Bearer từ file, không in token. Không sửa payload production:
vẫn gửi DICOM và metadata.json như Java. Hai study synthetic riêng, không sửa
study tham chiếu. Preflight 200/application/dicom+json; ingest 200 cả hai mẫu;
pending được lưu trước export và tồn tại qua restart.

Scheduler gửi thành công theo tiêu chí HTTP 2xx của contract; pending còn 0,
cả hai thư mục series đã cleanup. Store trả metadata rỗng ngay sau STOW rồi
hiển thị cả hai instance khi GET lại: có độ trễ lập chỉ mục, chưa đo ngưỡng
latency. Runner đầu exit 1 vì mảng metadata rỗng; không sửa production, đã hoàn
tất bằng delayed readback của đúng hai instance đã tạo.

WADO instance với Accept multipart/related trả 406 (cả có và không có
transfer-syntax=*); Accept application/dicom trả 200 với body multipart DICOM.
pydicom xác nhận Study/Series/SOP UID, transfer syntax và PixelData khớp local
cho cả JPEG Baseline và Explicit VR Little Endian. Pillow giải mã JPEG 8x8;
raw pixel 8x8/64 byte. JPEG frame local 200; raw frame local vẫn là giới hạn
404 đã ghi, không tính là pass. DB test đã drop và config credential đã dọn.

Kết quả tổng hợp `passed:true` tại
`target/step-13-logs/live-qa-store-2-attempt-3/result.json`, ghi rõ runner đầu
thất bại và bước xác minh bổ sung; `delayed-readback.json`,
`wado-accept-probes.json`, `*-local.dcm`, `*-qa.dcm`, `app-*.log` là bằng chứng.
Runner đã bổ sung chờ metadata tối đa 30 giây và Accept fallback khi 406 cho
lần chạy sau; chưa chạy lại nguyên runner sau hai thay đổi này. Không tạo thêm
study chỉ để kiểm tra runner. Hai study synthetic giữ trên store để review.
Phạm vi đạt: binary Windows/JVM/PostgreSQL thật → STORE_2 → readback → cleanup;
không thay bằng chứng Linux/Docker/soak hoặc khả năng tương thích store cũ.

### Gửi vào Study UID được người dùng chỉ định — 2026-11-06

Người dùng xác nhận study tồn tại và bổ sung `URL wadors` / `URL Stowrs` trong
STORE_2.md. Đã gọi đúng STOW URL kết thúc `/wado-rs/studies`; không thêm
`/studies` lần nữa. Runner hỗ trợ định dạng file mới và mode reference study.
Theo xác nhận trực tiếp của người dùng, không dùng QIDO rỗng để chặn gửi.
Không tạo thông tin bệnh nhân giả khi source metadata chưa đọc được.

Đã ingest JPEG và raw vào StudyInstanceUID được ghi trong STORE_2.md, cùng
một series mới `2.25.254197501482085026876857513389340949285`, hai SOP UID mới.
Scheduler dùng payload Java gồm cả metadata.json, pending về 0 và local series
đã cleanup. Chưa có readback sau thời gian chờ 30 giây: metadata 200/[],
WADO instance Accept application/dicom trả 404 cho cả hai ảnh.

Để thu response STOW, đã gửi lại đúng hai DICOM với cùng SOP UID (không sinh
SOP mới), payload diagnostic chỉ DICOM. Store trả HTTP 200/application/dicom+json,
Referenced SOP Sequence (00081199) liệt kê cả hai SOP; không có failed sequence
trong body. Không lấy diagnostic này làm bằng chứng payload production đã đổi.
Sau đó GET lại vẫn metadata rỗng/DICOM 404. Kết luận: đã gửi đúng URL/Study UID
và có STOW acknowledgment; chưa xác nhận store đã lưu/index vào study để đọc
lại. Không đánh dấu lượt này pass, không suy diễn chắc chắn nguyên nhân là
indexing. QIDO không liệt kê ảnh gốc nên cũng chưa xác minh được inventory gốc.

Bằng chứng tại `target/step-13-logs/store-2-reference-stow/`: result.json,
direct-stow-diagnostic.json, direct-same-instances-stow-response.body,
final-readback.json và các DICOM local. DB test/config credential đã dọn;
không gửi DELETE lên store. Lượt thành công trước với study synthetic riêng
vẫn là bằng chứng riêng, không thay kết quả reference-study chưa đạt này.

### Gửi trực tiếp, bỏ qua BE Rust — 2026-11-06

Theo yêu cầu người dùng, `python tools/send_key_image_direct.py` dùng HTTP
trực tiếp tới STOW URL trong STORE_2.md: không khởi động/gọi Rust, JVM hoặc
database; chỉ gửi hai DICOM, không metadata.json. Dùng lại chính Study/Series/SOP
UID và file của lượt reference-study, không tạo SOP mới, không xóa dữ liệu store.

STOW trả 200, body liệt kê cả hai SOP trong Referenced SOP Sequence. Sau chờ/poll
30 giây, cả hai metadata vẫn 200/[], WADO DICOM application/dicom trả 404.
Lượt direct chưa đạt readback; tool exit 0 chỉ thể hiện hoàn tất phép đo, không
phải assertion pass. Evidence ở `target/step-13-logs/direct-store-2/result.json`,
stow-response.body và các body GET.

Hiện tượng tái hiện khi bỏ qua tầng gửi Rust và bỏ metadata.json. Vì vẫn dùng
cùng DICOM đầu vào đã tạo trước đó, chưa loại trừ ảnh hưởng của nội dung DICOM,
auth/routing hoặc xử lý phía store. Không kết luận nguyên nhân chỉ từ HTTP 200.

### Yêu cầu bổ sung thuộc tính từ study gốc — 2026-11-06

Theo yêu cầu người dùng, đã GET study theo UID và includefield=all để lấy
PatientID/PatientName/StudyDate/StudyTime/AccessionNumber/Modality trước khi
gửi trực tiếp. API hiện trả 1 study, có PatientID nhưng không có 5 trường còn
lại; NumberOfStudyRelatedSeries=0 và NumberOfStudyRelatedInstances=0.
Series và series?includefield=all đều trả []; study metadata trả 404. Thử QIDO
prefix riêng và query series/instances không tìm được endpoint nguồn hữu dụng.
Chưa gửi lại file enriched, chưa tự điền giá trị bệnh nhân hoặc modality.
Cần metadata/DICOM gốc hoặc endpoint đọc instance nguồn để hoàn tất đủ 6 trường.
Evidence chỉ lưu local ignored ở `target/step-13-logs/key-image-source/`;
enrichment-preflight.json chỉ có trạng thái field/count, không có giá trị PHI.

### Auth mới / department02 và key image đủ thuộc tính nguồn — 2026-11-06

Sau khi người dùng cập nhật auth, QIDO study và series trả 200, 1 study và
1 series. Danh sách instance trả 1 instance với các tag nguồn đầy đủ;
instance /metadata bị timeout 15 giây nên dùng attributes từ instance listing.
PatientID, PatientName, StudyDate, StudyTime, Modality có giá trị; AccessionNumber
có tag nhưng rỗng. Sao chép nguyên trạng cả tag rỗng, không sinh accession giả.

`tools/send_enriched_key_image.py` tạo 1 JPEG DICOM từ file thử cũ, copy patient
group và study attributes từ instance nguồn, giữ StudyInstanceUID, tạo Series/SOP
UID mới cho file thay đổi để không ghi đè instance đã có. Assert lại file sau
khi ghi xác nhận cả 6 tag bằng nguồn. Gửi trực tiếp STOW department02, không
Rust/JVM/DB và không metadata.json. Store trả 200, response liệt kê SOP mới.

Chưa đạt readback: sau poll 45 giây không lấy được DICOM; QIDO study-series vẫn
1 series, query series mới trả [] (200). Vì vậy chưa kết luận thiếu các trường
này là nguyên nhân duy nhất của lỗi lưu/index trước đó. Không xóa remote data.
Evidence local ignored tại `target/step-13-logs/enriched-direct-store-2/` gồm
result.json, jpeg-enriched.dcm, jpeg-enriched.dicom.json, stow-response.json,
one-key-image.multipart và QIDO results. Metadata có PHI, không đưa giá trị vào
version control hoặc log chat. Lượt enriched chưa pass; không sửa contract Rust.
