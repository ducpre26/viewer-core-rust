# Step 4 — baseline UC-009

Java revision: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`.
Đối chiếu nguồn: AdminAPI, Client/Permission DTO, ClientServiceImpl,
PermissionCrudServiceImpl, ClientPermissionServiceImpl, IBaseService,
ICreateService, IUpdateService, IDeleteService, APIResponse*, ErrorResponse,
GlobalExceptionHandler, PasswordEncoder, Util.toCleanText và db/init.sql.
Không có job của tính năng này. Đây là expected contract suy ra từ source,
chưa phải bản ghi chạy Spring HTTP.

Prefix `/ws/rest/v1/admin`; tất cả route yêu cầu Basic admin cấu hình,
client thường (kể cả có permission ADMIN) không được gọi.

| Fixture / thao tác | Expected |
|---|---|
| GET client/permission | 200, header.code 302, message `Found N clients/permissions`, offset/limit/totalRecords; body array |
| GET client/{id} | 200, code 200, Success; body id/ipAllowed/permissionIds; không password |
| GET permission/{id} | 200, code 200, Success; body id/name/description |
| GET/PUT tài nguyên không tồn tại | 404, body rỗng |
| POST client `{id,password,ipAllowed}` | 201, code 201, Created, body id; trim password/IP, BCrypt cost 8 |
| POST permission `{id,name,description}` | 201, code 201, Created, body id; name không trim |
| POST cùng ID đã tồn tại | JPA save/merge cập nhật entity, vẫn 201; không coi là duplicate-ID error |
| PUT client/permission | 200, Updated, body id; trường null không thay đổi; client trim chuỗi, blank thành null |
| DELETE client/permission có tồn tại | 200, Deleted, body id; FK ON DELETE CASCADE dọn mapping |
| DELETE client/permission không tồn tại | 404, ErrorResponse NOT_FOUND, `Entity not found with id: ID` |
| Thiếu id/password/name bắt buộc khi tạo | 400, ErrorResponse VALIDATION_ERROR |
| Permission name trùng / vượt độ dài DB | lỗi DB, HTTP 500; không thay đổi dữ liệu |
| GET client/{id}/permission | 200, code 200, `Lấy danh sách quyền thành công`; DTO clientId/permissionId/permissionName/permissionDescription |
| POST mapping `{permissionId}` hợp lệ | 201, code 200, `Thêm quyền thành công`, body permissionId |
| POST mapping trùng / client hoặc permission thiếu / permissionId blank | 500, code 500, message nghiệp vụ, body null |
| DELETE mapping | 200, code 200, `Xóa quyền thành công`, body clientId; cặp không tồn tại vẫn thành công |

Pagination: offset mặc định 0, limit 100; page=max(0,offset/max(1,limit)),
query bắt đầu tại page*limit; orderBy bị bỏ qua; limit <= 0 là 400.
Không cam kết thứ tự khi Java không có ORDER BY.
Envelope thành công có header.datetime; lỗi toàn cục có timestamp.

Khác UC-009 A1: remove mapping không kiểm tra sự tồn tại. Khác trực giác 400/404:
IllegalPropertyException extends ViewerException extends RuntimeException,
không phải IllegalArgumentException; catch Exception trong mapping trả 500.
Giữ code Java theo quy tắc baseline trong README step; không sửa spec ở step này.
Không đưa SQL có giá trị tham số/credential vào phản hồi Rust khi lỗi DB:
message driver cụ thể phụ thuộc môi trường và có thể lộ dữ liệu.
