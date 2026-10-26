# Step 5 — baseline UC-008

Java revision: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`.
Đối chiếu ConfigPublicAPI, phần config của AdminAPI, ConfigServiceImpl,
ConfigRepository, Config entity/DTO, Util.toCleanText, base service/response,
SessionPermissionEvaluator, PermissionCheckService, WebRequestFilter,
GlobalExceptionHandler, db/init.sql và ehcache.xml. Không có job.
Hai file Java ConfigPublicAPITest/ConfigServiceImplTest hiện rỗng; expected dưới
đây suy ra từ source, chưa phải kết quả chạy Spring HTTP.

Hai prefix: `/ws/rest/v1/admin/config` và
`/ws/rest/v1/session/{sessionId}/config`.
Admin yêu cầu Basic admin cấu hình; client có permission ADMIN không thay thế
Basic admin. Phạm vi phiên yêu cầu session hợp lệ (kể cả khi client security
disabled), và session-key đúng nếu là phiên chia sẻ.

| Fixture / thao tác (cả hai prefix) | Expected |
|---|---|
| GET collection | HTTP 200; header.code 302, `Found N configs`, offset/limit/totalRecords; body array |
| GET /{id} | 200, code 200, `Success`; body id/value/description/datatype (null được serialize) |
| GET/PUT ID không tồn tại | 404, body rỗng |
| POST `{id:"step5-overlay",value:"  fixture  "}` | 201, code 201, `Created`, body ID; DB value `fixture`, description null, datatype TEXT |
| POST cùng ID | JPA save/merge thay toàn bộ giá trị, vẫn 201; cache chứa giá trị mới |
| POST thiếu/null id hoặc value, value blank sau Java trim | 400, VALIDATION_ERROR, `must not be null` cho từng trường lỗi |
| POST id rỗng/blank | Được phép: @NotNull, không @NotBlank và không trim ID |
| POST datatype TEXT/INTEGER/LIST hoặc null | Chấp nhận; null/missing mặc định TEXT; không validate nội dung value theo datatype |
| POST datatype khác enum | Lỗi đọc JSON của Spring (HTTP 500 qua catch Exception); không ghi DB |
| POST description | Java trim U+0000..U+0020; blank thành null; NBSP giữ nguyên |
| PUT `{id,value,description,datatype}` | 200, code 200, `Updated`, body ID; chỉ apply trường non-null, không trim/validate enum; null/missing giữ giá trị cũ |
| PUT thiếu/null id | Không có @NotNull: cache key/repository báo IllegalArgumentException, HTTP 400 BAD_REQUEST |
| DELETE /{id} tồn tại | 200, code 200, `Deleted`, body ID; DB xóa; cache evict |
| DELETE /{id} thiếu | 404, NOT_FOUND, `Entity not found with id: ID` |
| Vượt id varchar(64) / datatype varchar(32) | 500 INTERNAL_ERROR, DB không đổi; cache giữ giá trị đã commit |

Pagination giống Step 4: offset=0, limit=100, page=max(0,offset/max(1,limit)),
bắt đầu tại page*limit; orderBy bị bỏ qua; limit<=0 là 400. Không cam kết thứ tự.
Response datetime/timestamp biến thiên. Lỗi DB dùng message an toàn như Step 4,
không trả message driver chứa tham số.

Quyền phiên: GET/list/DELETE không có @PreAuthorize. POST/PUT cần permission
MODIFY_OVERLAY_CONFIG hoặc ADMIN trong snapshot session, lấy sessionId từ path
(không lấy query). Thiếu quyền với anonymous session trả 401 theo security
entry point Step 3a; client security disabled bypass permission. Basic admin
không thay thế snapshot permission ở phạm vi phiên.

Cache: lookup theo ID; TTL 600 phút, không cache null; list đọc DB. Create put,
update/delete evict sau ghi thành công. Rust dùng Moka đã có, một cache dùng
chung cho cả admin và session; đồng bộ lookup/write để cache fill cũ không
ghi đè invalidation. Dung lượng 10 entry theo heap cache Java; Java còn offheap
50MB nên tổng số entry lưu thực tế khác, không đổi HTTP/DB contract.

Đối chiếu UC-008: BR-001 chỉ bắt buộc khi tạo (PUT cố ý cho phép partial update);
BR-002 chỉ áp dụng POST/PUT qua phiên. DELETE không yêu cầu quyền sửa overlay
theo code nguồn. Giữ hành vi này theo quy tắc migration; không sửa use case.
Schema CONFIG: id varchar(64) PK, value text NOT NULL, description text nullable,
datatype varchar(32) nullable; không seed config ở step này.
