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
`74a7b414ab0193bca3df9b48f8026f724991f2e8`. Chưa có fixture/runtime
UC-006 chứng minh parity; kết quả đọc source không thay thế kiểm thử thực tế.

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

- [ ] Thiết kế facade parse/bổ sung/validate/ghi DICOM theo thứ tự Java nêu trên; không coi create-key-image là contract tạo nguyên lô; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping STUDY_KEY_IMAGE và migration/repository SeaORM theo DDL Java; tái sử dụng SESSION, lưu file key image và typed TOML settings; không thêm CONFIG setting mà Java không dùng.
- [ ] Parse multipart và validate đúng nhánh Java; không tự thêm study scope, đồng nhất study hoặc required attributes ngoài baseline.
- [ ] Tạo/cập nhật pending theo study trước khi ghi DICOM, giữ số ảnh, retry=1 và description theo nhánh Java.
- [ ] Java facade resolve bulk, bổ sung/validate thuộc tính và ghi lần lượt DICOM theo source; sau đó ghi metadata/response URI.
- [ ] Scheduler lấy tối đa 100 batch, retry `<= 3`, gửi STOW khi feature bật.
- [ ] Giữ payload STOW và tiêu chí HTTP 2xx của Java, kể cả metadata.json và lỗi đọc file chỉ log; kiểm chứng bằng fixture.
- [ ] Cleanup đúng phạm vi/thứ tự Java: ingest dọn request spool; STOW thành công xóa record rồi thư mục series; không rollback pending/file đã ghi khi ingest lỗi.
- [ ] Port các nhánh WADO series/metadata/frame local liên quan key image đang pending theo Java.
- [ ] Áp dụng [mục 5.7](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership) cho JNI create/validate DICOM; cleanup không xóa file Java còn sử dụng.

## Kiểm tra hoàn thành

- [ ] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] Dùng UC-006 main flow, A1..A3, BR-001..BR-004 làm chỉ mục coverage, expected result theo contract Java ở trên khi khác UC: retry 1/3/4, 100/101 record, feature tắt, hạn mức pending, batch nhiều study/series, lỗi giữa lô, STOW 2xx có partial failure, lỗi đọc file và cleanup; kiểm tra đọc local trước khi xuất.
- [ ] Cancellation khi facade đang dùng file: không cleanup sớm; record/temp state vẫn được kiểm chứng theo contract.
- [ ] Đạt khi: output/response và pending/retry/cleanup state khớp baseline Java, bao gồm nhánh lỗi; không dùng hậu điều kiện UC mâu thuẫn source để đổi hành vi. Kiểm chứng DICOM của luồng thành công bằng corpus tính năng.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Rà soát source và sửa contract/checklist theo xác nhận người dùng: Java là chuẩn; ghi rõ thứ tự pending trước DICOM, STOW/retry/cleanup và dependency đọc local. Chưa triển khai tính năng.
- Tiếp theo: Thu baseline runtime/fixture synthetic đúng các nhánh Java đã ghi, chốt ranh giới facade rồi triển khai theo checklist hiện có khi được yêu cầu.
- Vướng mắc: Chưa thu baseline runtime UC-006; không còn yêu cầu lựa chọn lại nghiệp vụ từ các khác biệt UC/source đã nêu. Step 6/7 không phải dependency trực tiếp; các kiểm chứng Step 12 người dùng đã hoãn không chặn Step 13 và không được tính là đạt.
- Kiểm tra gần nhất: Đối chiếu tĩnh route/service/job/DDL và tài liệu; chưa chạy test tính năng hoặc xác nhận parity Java/Rust.
- Cập nhật: 2026-11-05 — Codex sửa tài liệu theo contract Java; giữ `not_started`, không đánh dấu checkbox hoàn tất.
