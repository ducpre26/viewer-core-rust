# UC verification checklist — draft

Extracted assertions from UC documents, not observed Java results. Each unchecked item requires concrete synthetic input, fixture ID, expected HTTP/body and relevant DB/file effects before independent execution.

## UC-001-khoi-tao-phien-xem-anh-2d

### Main Success Scenario

- [ ] Hệ thống client PACS cung cấp kho DICOM, danh sách study, người dùng và thời hạn mong muốn.
- [ ] Hệ thống xác thực client và kiểm tra giới hạn địa chỉ mạng nếu được cấu hình.
- [ ] Hệ thống lấy các quyền đã gán cho client.
- [ ] Hệ thống chuẩn hóa thời hạn và tạo phiên chứa các study cùng quyền hiệu lực.
- [ ] Hệ thống trả về liên kết Viewer của phiên.
### Alternative Flows

#### A1: Client không hợp lệ
**Trigger:** Không thể xác thực client hoặc địa chỉ mạng không được phép (step 2)  
**Flow:**
- [ ] Hệ thống từ chối yêu cầu tạo phiên.
- [ ] Use case ends.
#### A2: Dữ liệu phiên thiếu
**Trigger:** Thiếu kho DICOM hoặc danh sách study (step 1)  
**Flow:**
- [ ] Hệ thống báo dữ liệu không hợp lệ.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Phiên 2D có thời hạn tồn tại trong vùng lưu trữ phiên và liên kết Viewer được cấp.
#### Failure Postconditions
- [ ] Không có phiên mới được tạo từ yêu cầu không hợp lệ.
### Business Rules

#### BR-001: Dữ liệu bắt buộc
Kho DICOM và tập study là bắt buộc khi tạo phiên.
#### BR-002: Giới hạn thời hạn
Thời hạn không dương hoặc vượt giới hạn cấu hình được thay bằng thời hạn mặc định của hệ thống.
#### BR-003: Quyền theo client
Quyền của phiên được chụp từ các quyền hiện được gán cho client tại thời điểm tạo phiên.

## UC-002-quan-ly-va-chia-se-phien-2d

### Main Success Scenario

- [ ] Người dùng Viewer chọn thao tác xem, cập nhật, chia sẻ hoặc kết thúc phiên.
- [ ] Hệ thống kiểm tra phiên và khóa chia sẻ nếu phiên yêu cầu khóa.
- [ ] Khi cập nhật, hệ thống thay đổi danh sách study của phiên ngắn hạn.
- [ ] Khi chia sẻ, hệ thống xác nhận phiên nguồn không phải là phiên chia sẻ và xác định thời hạn hiệu lực.
- [ ] Hệ thống tạo một phiên chia sẻ dài hạn với tùy chọn ẩn danh và khóa truy cập đã chọn.
- [ ] Khi kết thúc phiên, hệ thống xóa phiên khỏi cả vùng lưu trữ ngắn hạn và dài hạn.
- [ ] Hệ thống trả thông tin phiên hoặc liên kết chia sẻ tương ứng.
### Alternative Flows

#### A1: Phiên hoặc khóa chia sẻ không hợp lệ
**Trigger:** Phiên không tồn tại hoặc khóa cung cấp không khớp (step 2)  
**Flow:**
- [ ] Hệ thống từ chối truy cập phiên.
- [ ] Use case ends.
#### A2: Phiên cập nhật không phải phiên ngắn hạn
**Trigger:** Không tìm thấy phiên trong vùng lưu trữ ngắn hạn khi cập nhật (step 3)  
**Flow:**
- [ ] Hệ thống không cập nhật danh sách study.
- [ ] Use case ends.
#### A3: Chia sẻ lại phiên chia sẻ
**Trigger:** Phiên nguồn đã là một phiên chia sẻ (step 4)  
**Flow:**
- [ ] Hệ thống không tạo thêm phiên chia sẻ.
- [ ] Use case ends.
#### A4: Thời hạn chia sẻ ngoài phạm vi
**Trigger:** Thời hạn chia sẻ không dương hoặc vượt giới hạn cấu hình (step 4)  
**Flow:**
- [ ] Hệ thống đặt thời hạn phiên chia sẻ thành một giờ kể từ thời điểm tạo.
- [ ] Use case continues at step 5.
#### A5: Không tạo được phiên chia sẻ
**Trigger:** Dịch vụ phiên không thể lưu phiên chia sẻ (step 5)  
**Flow:**
- [ ] Hệ thống không cấp liên kết chia sẻ.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Phiên được đọc hoặc thay đổi theo yêu cầu; nếu chia sẻ thành công, một phiên chia sẻ riêng tồn tại.
#### Failure Postconditions
- [ ] Không cấp quyền truy cập cho phiên hoặc khóa chia sẻ không hợp lệ.
### Business Rules

#### BR-001: Khóa phiên chia sẻ
Phiên có share key chỉ chấp nhận truy cập khi giá trị được cung cấp khớp với share key của phiên.
#### BR-002: Thời hạn chia sẻ bắt buộc
Yêu cầu chia sẻ phải cung cấp thời lượng hết hạn.
#### BR-003: Phạm vi cập nhật phiên
Thao tác cập nhật chỉ thay đổi danh sách study và chỉ áp dụng cho phiên đang có trong vùng lưu trữ ngắn hạn.
#### BR-004: Không chia sẻ lại
Một phiên đã là phiên chia sẻ không thể được dùng để tạo thêm phiên chia sẻ.
#### BR-005: Chuẩn hóa thời hạn chia sẻ
Thời hạn không dương hoặc vượt giới hạn cấu hình được thay bằng một giờ.
#### BR-006: Định dạng khóa chia sẻ
Khi được yêu cầu, khóa của phiên chia sẻ gồm sáu chữ số.
#### BR-007: Dọn phiên dài hạn hết hạn
Bộ lập lịch xóa các phiên dài hạn đã hết hạn vào nửa đêm mỗi ngày theo múi giờ Asia/Ho_Chi_Minh.

## UC-003-tra-cuu-study-dicom

### Main Success Scenario

- [ ] Người dùng Viewer cung cấp định danh phiên, Patient ID, các tiêu chí tùy chọn và thông tin phân trang.
- [ ] Hệ thống kiểm tra phiên và chuẩn hóa vị trí bắt đầu cùng giới hạn kết quả.
- [ ] Hệ thống chuyển tiêu chí tra cứu tới kho DICOM.
- [ ] Kho DICOM trả các study phù hợp.
- [ ] Hệ thống chuẩn hóa kết quả và trả danh sách study.
### Alternative Flows

#### A1: Không có kết quả
**Trigger:** Kho DICOM không trả study phù hợp (step 4)  
**Flow:**
- [ ] Hệ thống trả trạng thái không có nội dung.
- [ ] Use case ends.
#### A2: Phiên không tồn tại
**Trigger:** Không tìm thấy phiên được cung cấp (step 2)  
**Flow:**
- [ ] Hệ thống trả trạng thái không có nội dung.
- [ ] Use case ends.
#### A3: Thiếu dữ liệu tra cứu bắt buộc
**Trigger:** Thiếu định danh phiên hoặc Patient ID (step 1)  
**Flow:**
- [ ] Hệ thống từ chối yêu cầu tra cứu.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Người dùng nhận được danh sách study phù hợp; dữ liệu nghiệp vụ không bị thay đổi.
#### Failure Postconditions
- [ ] Không có study hoặc phiên nào được tạo hay sửa.
### Business Rules

#### BR-001: Giới hạn mặc định
Khi actor không chỉ định, kết quả bắt đầu từ vị trí đầu tiên và giới hạn mặc định là 100 study. Vị trí âm được chuẩn hóa về 0; giới hạn không dương được chuẩn hóa thành 20.
#### BR-002: Dữ liệu tra cứu bắt buộc
Định danh phiên phải khác rỗng và Patient ID là bắt buộc.
#### BR-003: Tham số sắp xếp chưa được áp dụng
Giá trị sắp xếp có thể được tiếp nhận nhưng không được dùng để thay đổi truy vấn hoặc thứ tự kết quả.

## UC-004-truy-xuat-du-lieu-anh-dicom

### Main Success Scenario

- [ ] Người dùng Viewer xác định study và phần dữ liệu cần truy xuất.
- [ ] Hệ thống kiểm tra quyền truy cập phiên.
- [ ] Hệ thống chuyển yêu cầu cùng thông tin phiên tới kho DICOM.
- [ ] Kho DICOM trả dữ liệu và thông tin mô tả phản hồi.
- [ ] Hệ thống truyền dữ liệu phù hợp về Viewer.
### Alternative Flows

#### A1: Phiên hoặc study không thuộc phạm vi truy cập
**Trigger:** Không tìm thấy phiên hoặc study không nằm trong danh sách study của phiên (step 2)  
**Flow:**
- [ ] Hệ thống từ chối truy xuất.
- [ ] Use case ends.
#### A2: Kho DICOM từ chối hoặc không có dữ liệu
**Trigger:** Kho DICOM không xác thực yêu cầu hoặc không tìm thấy đối tượng (step 4)  
**Flow:**
- [ ] Hệ thống trả trạng thái lỗi tương ứng và không tạo dữ liệu thay thế.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Dữ liệu DICOM yêu cầu được truyền về Viewer mà không thay đổi kho nguồn.
#### Failure Postconditions
- [ ] Không có dữ liệu DICOM nghiệp vụ nào được ghi bởi hệ thống.
### Business Rules

#### BR-001: Phạm vi truy xuất
Hệ thống hỗ trợ dữ liệu ở mức study, series, instance, frame, thumbnail, metadata và bulk data khi kho nguồn cung cấp.
#### BR-002: Phạm vi study của phiên
Chỉ được truy xuất study có trong danh sách study của phiên đang sử dụng.

## UC-005-tai-noi-dung-dicom

### Main Success Scenario

- [ ] Người dùng Viewer chọn study, series hoặc instance và kiểu tải.
- [ ] Hệ thống lấy các instance tương ứng từ kho DICOM.
- [ ] Hệ thống đóng gói hoặc truyền dữ liệu theo kiểu tải đã chọn.
- [ ] Hệ thống phát luồng tệp về cho người dùng.
### Alternative Flows

#### A1: Kiểu tải không được nhận diện
**Trigger:** Không cung cấp kiểu tải hoặc giá trị kiểu tải không thể được diễn giải (step 1)  
**Flow:**
- [ ] Hệ thống diễn giải yêu cầu thành kiểu luồng nhị phân mặc định.
- [ ] Hệ thống từ chối kiểu này vì không thuộc tập kiểu được hỗ trợ tại phạm vi tải study, series hoặc instance.
- [ ] Use case ends.
#### A2: Không lấy được dữ liệu nguồn
**Trigger:** Kho DICOM không trả được dữ liệu yêu cầu (step 2)  
**Flow:**
- [ ] Hệ thống kết thúc luồng tải với lỗi.
- [ ] Use case ends.
#### A3: Study không thuộc phiên
**Trigger:** Study được chọn không nằm trong danh sách study của phiên (step 2)  
**Flow:**
- [ ] Hệ thống không cung cấp nội dung DICOM.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Người dùng nhận được nội dung DICOM ở phạm vi và định dạng đã chọn.
#### Failure Postconditions
- [ ] Không có thay đổi đối với phiên hoặc kho DICOM.
### Business Rules

#### BR-001: Kiểu tải mặc định
Khi không chỉ định hoặc không nhận diện được kiểu tải, hệ thống chuyển giá trị thành dạng luồng nhị phân; các thao tác tải study, series và instance hiện tại đều từ chối dạng này.
#### BR-002: Phạm vi study của phiên
Chỉ được tải study có trong danh sách study của phiên đang sử dụng.
#### BR-003: Kiểu tải theo phạm vi
Tải study và instance chỉ chấp nhận DICOM hoặc JPEG; tải series chấp nhận DICOM, JPEG hoặc MP4.

## UC-006-tiep-nhan-va-xuat-key-image

### Main Success Scenario

- [ ] Người dùng Viewer gửi metadata DICOM và dữ liệu ảnh của key image.
- [ ] Hệ thống kiểm tra cấu trúc lô và các thuộc tính DICOM bắt buộc.
- [ ] Hệ thống bổ sung các thuộc tính có thể suy ra và lưu tạm các đối tượng DICOM.
- [ ] Hệ thống ghi nhận lô đang chờ xuất.
- [ ] Bộ lập lịch chọn lô chưa thành công còn trong giới hạn thử lại.
- [ ] Hệ thống gửi lô tới kho DICOM đích.
- [ ] Khi kho chấp nhận, hệ thống xóa bản ghi chờ và dữ liệu tạm.
### Alternative Flows

#### A1: Nội dung gửi lên không hợp lệ
**Trigger:** Thiếu ranh giới lô, thuộc tính DICOM hoặc dữ liệu ảnh bắt buộc (step 2)  
**Flow:**
- [ ] Hệ thống từ chối lô và dọn thư mục tạm của yêu cầu.
- [ ] Use case ends.
#### A2: Kho DICOM từ chối lô
**Trigger:** Kho DICOM không chấp nhận lần gửi (step 6)  
**Flow:**
- [ ] Hệ thống tăng số lần thử lại và lưu lý do lỗi.
- [ ] Use case ends.
#### A3: Vượt giới hạn key image của study
**Trigger:** Tổng số key image hiện có và mới gửi vượt giới hạn cấu hình của study (step 2)  
**Flow:**
- [ ] Hệ thống từ chối lô key image mới.
- [ ] Hệ thống dọn dữ liệu tạm của yêu cầu.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Key image đã được kho DICOM chấp nhận và dữ liệu tạm tương ứng đã được dọn.
#### Failure Postconditions
- [ ] Dữ liệu tạm của một yêu cầu tiếp nhận thất bại được dọn; lô xuất thất bại còn được theo dõi nếu chưa hết số lần thử.
### Business Rules

#### BR-001: Giới hạn thử lại
Bộ lập lịch chỉ chọn lô chưa thành công có số lần thử lại nhỏ hơn hoặc bằng 3.
#### BR-002: Xuất theo cấu hình
Việc gửi lô chờ tới kho DICOM chỉ chạy khi tùy chọn gửi key image được bật.
#### BR-003: Giới hạn key image theo study
Tổng số key image đã ghi nhận và mới tiếp nhận của một study không được vượt giới hạn cấu hình của hệ thống.
#### BR-004: Giới hạn lô gửi mỗi lượt
Mỗi lượt chạy, Bộ lập lịch xử lý tối đa 100 lô key image đang chờ gửi.

## UC-007-quan-ly-phien-dung-hinh-3d

### Main Success Scenario

- [ ] Người dùng Viewer cung cấp phiên 2D và thông tin dữ liệu cần dựng hình.
- [ ] Hệ thống cấp một tiến trình dựng hình và tạo phiên 3D.
- [ ] Hệ thống trả liên kết kết nối thời gian thực.
- [ ] Người dùng Viewer mở kết nối bằng định danh phiên 2D và phiên 3D.
- [ ] Hệ thống kiểm tra hai phiên rồi chuyển tiếp thông điệp giữa Viewer và tiến trình dựng hình.
- [ ] Khi kết nối kết thúc, hệ thống giải phóng phiên và tài nguyên theo trạng thái sử dụng.
### Alternative Flows

#### A1: Thiếu tài nguyên 3D
**Trigger:** Không còn bộ nhớ, cổng hoặc tiến trình khả dụng (step 2)  
**Flow:**
- [ ] Hệ thống không tạo phiên 3D và báo dịch vụ tạm không sẵn sàng.
- [ ] Use case ends.
#### A2: Phiên 2D không hợp lệ khi tạo phiên 3D
**Trigger:** Không tìm thấy phiên 2D nguồn (step 2)  
**Flow:**
- [ ] Hệ thống không tạo phiên 3D và từ chối yêu cầu.
- [ ] Use case ends.
#### A3: Phiên không hợp lệ khi kết nối
**Trigger:** Thiếu hoặc không tìm thấy phiên 2D hay phiên 3D (step 5)  
**Flow:**
- [ ] Hệ thống đóng kết nối.
- [ ] Use case ends.
#### A4: Tra cứu trạng thái phiên hoặc tài nguyên
**Trigger:** Ứng dụng gọi API yêu cầu danh sách phiên, chi tiết phiên hoặc các cổng khả dụng (step 1)  
**Flow:**
- [ ] Hệ thống trả danh sách phiên, phiên theo định danh hoặc danh sách cổng khả dụng tương ứng.
- [ ] Use case ends.
#### A5: Chủ động xóa phiên
**Trigger:** Ứng dụng gọi API yêu cầu xóa một phiên (step 1)  
**Flow:**
- [ ] Nếu phiên tồn tại, hệ thống trả tiến trình về nhóm tài nguyên khi có thể và xóa phiên khỏi registry.
- [ ] Hệ thống trả kết quả thành công cho định danh được yêu cầu.
- [ ] Use case ends.
#### A6: Đặt lại registry tài nguyên
**Trigger:** Ứng dụng gọi API yêu cầu buộc đặt lại tài nguyên (step 1)  
**Flow:**
- [ ] Hệ thống xóa registry phiên và khởi tạo lại danh sách cổng khả dụng.
- [ ] Hệ thống trả danh sách cổng khả dụng sau thao tác.
- [ ] Use case ends.
#### A7: Định danh phiên trống
**Trigger:** Định danh phiên để tra cứu hoặc xóa bị trống (step 1)  
**Flow:**
- [ ] Hệ thống từ chối thao tác quản lý phiên.
- [ ] Use case ends.
#### A8: Tra cứu phiên không tồn tại
**Trigger:** Không tìm thấy phiên được yêu cầu khi tra cứu chi tiết (step 1)  
**Flow:**
- [ ] Hệ thống từ chối yêu cầu tra cứu chi tiết.
- [ ] Use case ends.
#### A9: Xóa phiên không tồn tại
**Trigger:** Không tìm thấy phiên được yêu cầu khi xóa (step 1)  
**Flow:**
- [ ] Hệ thống không thay đổi registry và vẫn trả kết quả thành công cho định danh được yêu cầu.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Phiên 3D hoạt động trong thời gian kết nối và tài nguyên được trả lại sau khi kết thúc.
#### Failure Postconditions
- [ ] Không giữ lại tài nguyên cho phiên 3D không được thiết lập thành công.
### Business Rules

#### BR-001: Ghép cặp phiên
Kết nối dựng hình chỉ được thiết lập khi cả phiên 2D và phiên 3D được xác định và tồn tại.
#### BR-002: Thu hồi tiến trình
Tiến trình đã kết thúc được kiểm tra định kỳ và thu hồi khỏi nhóm tài nguyên.
#### BR-003: Chu kỳ thu hồi tiến trình
Bộ lập lịch kiểm tra và thu hồi tiến trình đã kết thúc mỗi phút.
#### BR-004: Quản lý tài nguyên qua API
Ứng dụng gọi API có thể xem phiên 3D, xem cổng khả dụng, xóa một phiên hoặc đặt lại registry phiên và danh sách cổng. Thao tác đặt lại registry không trực tiếp dừng các tiến trình 3D đang chạy.

## UC-008-quan-ly-cau-hinh-viewer

### Main Success Scenario

- [ ] Actor chọn xem danh sách, xem chi tiết, tạo, cập nhật hoặc xóa cấu hình.
- [ ] Hệ thống kiểm tra quyền áp dụng cho thao tác.
- [ ] Hệ thống kiểm tra dữ liệu cấu hình.
- [ ] Hệ thống thực hiện thao tác và trả kết quả.
### Alternative Flows

#### A1: Thiếu quyền sửa cấu hình
**Trigger:** Người dùng Viewer không có quyền sửa overlay khi tạo hoặc cập nhật (step 2)  
**Flow:**
- [ ] Hệ thống từ chối thay đổi.
- [ ] Use case ends.
#### A2: Không tìm thấy cấu hình
**Trigger:** Cấu hình cần xem hoặc cập nhật không tồn tại (step 3)  
**Flow:**
- [ ] Hệ thống báo không tìm thấy.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Cấu hình được đọc hoặc trạng thái lưu trữ phản ánh thay đổi đã yêu cầu.
#### Failure Postconditions
- [ ] Không áp dụng thay đổi khi actor thiếu quyền hoặc cấu hình đích không tồn tại.
### Business Rules

#### BR-001: Thuộc tính bắt buộc
Định danh và giá trị cấu hình là bắt buộc khi tạo cấu hình.
#### BR-002: Quyền sửa overlay
Người dùng phiên phải có quyền MODIFY_OVERLAY_CONFIG hoặc quyền ADMIN để tạo hay cập nhật cấu hình qua phạm vi phiên.

## UC-009-quan-tri-client-va-quyen

### Main Success Scenario

- [ ] Quản trị viên chọn tài nguyên client, quyền hoặc ánh xạ client–quyền.
- [ ] Quản trị viên chọn xem, tạo, cập nhật, xóa hoặc gán/bỏ gán.
- [ ] Hệ thống kiểm tra sự tồn tại và dữ liệu bắt buộc.
- [ ] Hệ thống thực hiện thao tác.
- [ ] Hệ thống trả trạng thái hoặc dữ liệu quản trị mới nhất.
### Alternative Flows

#### A1: Client hoặc quyền không tồn tại
**Trigger:** Tài nguyên tham chiếu không tồn tại (step 3)  
**Flow:**
- [ ] Hệ thống không tạo hay xóa ánh xạ.
- [ ] Use case ends.
#### A2: Dữ liệu tạo không hợp lệ
**Trigger:** Thiếu định danh, bí mật client, tên quyền hoặc định danh quyền (step 3)  
**Flow:**
- [ ] Hệ thống từ chối thay đổi.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Client, quyền hoặc ánh xạ được lưu đúng theo thao tác quản trị.
- [ ] Khi client hoặc quyền bị xóa, mọi ánh xạ client–quyền tham chiếu tới đối tượng đó cũng bị xóa.
#### Failure Postconditions
- [ ] Không tạo ánh xạ mồ côi tới client hoặc quyền không tồn tại.
### Business Rules

#### BR-001: Truy cập quản trị
Chỉ principal mang vai trò quản trị mới được thực hiện các thao tác của vùng quản trị.
#### BR-002: Tính duy nhất của quyền
Tên quyền phải duy nhất trong danh mục quyền.
#### BR-003: Ánh xạ không trùng
Một cặp client và quyền chỉ xuất hiện tối đa một lần.
#### BR-004: Xóa lan truyền ánh xạ quyền
Xóa client hoặc quyền đồng thời xóa mọi ánh xạ client–quyền liên quan.

## UC-010-phan-tich-bat-thuong-bang-ai

### Main Success Scenario

- [ ] Người dùng Viewer cung cấp dữ liệu ca chụp và tên model.
- [ ] Hệ thống tìm cấu hình model tương ứng.
- [ ] Hệ thống gửi yêu cầu phân tích tới máy chủ AI.
- [ ] Máy chủ AI trả kết quả study và series.
- [ ] Hệ thống chuẩn hóa kết quả thành trạng thái thành công và trả về Viewer.
### Alternative Flows

#### A1: Thiếu hoặc không tìm thấy model
**Trigger:** Tên model trống hoặc không có model tương ứng (step 2)  
**Flow:**
- [ ] Hệ thống từ chối yêu cầu phân tích.
- [ ] Use case ends.
#### A2: Máy chủ AI không trả kết quả
**Trigger:** Kết quả từ máy chủ AI rỗng (step 4)  
**Flow:**
- [ ] Hệ thống báo suy luận thất bại.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Viewer nhận kết quả phân tích gắn với model được chọn.
#### Failure Postconditions
- [ ] Không có model hay dữ liệu nghiệp vụ bền vững bị thay đổi.
### Business Rules

#### BR-001: Chọn model bắt buộc
Tên model không được để trống và phải khớp một model đã đăng ký.

## UC-011-phan-doan-anh-bang-ai

### Main Success Scenario

- [ ] Người dùng Viewer cung cấp phiên, model và dữ liệu study/series cần phân đoạn.
- [ ] Hệ thống lấy thông tin kho DICOM từ phiên và tạo tác vụ tại máy chủ AI.
- [ ] Người dùng Viewer yêu cầu trạng thái hoặc thống kê bằng định danh tác vụ.
- [ ] Hệ thống lấy trạng thái hoặc thống kê từ máy chủ AI.
- [ ] Khi tác vụ hoàn tất, người dùng yêu cầu kết quả.
- [ ] Hệ thống tải kết quả và lưu vào vùng dữ liệu study/series dùng cho dựng hình.
- [ ] Hệ thống trả vị trí kết quả đã lưu.
### Alternative Flows

#### A1: Phiên hoặc model không hợp lệ
**Trigger:** Không tìm thấy phiên hoặc model được chọn (step 2)  
**Flow:**
- [ ] Hệ thống không tạo tác vụ phân đoạn.
- [ ] Use case ends.
#### A2: Kết quả không sẵn sàng
**Trigger:** Máy chủ AI trả lỗi hoặc nội dung kết quả rỗng (step 6)  
**Flow:**
- [ ] Hệ thống không tạo tệp kết quả mới.
- [ ] Use case ends.
#### A3: Kết quả đã tồn tại
**Trigger:** Tệp kết quả của tác vụ đã có tại vị trí đích (step 6)  
**Flow:**
- [ ] Hệ thống bỏ qua tải lại và dùng vị trí hiện có.
- [ ] Use case continues at step 7.
### Postconditions

#### Success Postconditions
- [ ] Kết quả phân đoạn tồn tại trong vùng dữ liệu của đúng study và series.
#### Failure Postconditions
- [ ] Không tạo tệp kết quả một phần khi máy chủ AI không cung cấp nội dung hợp lệ.
### Business Rules

#### BR-001: Định danh dữ liệu bắt buộc
Study Instance UID và Series Instance UID phải có trước khi lưu kết quả phân đoạn.
#### BR-002: Không tải trùng
Kết quả hiện có của cùng tác vụ không bị ghi đè.

## UC-012-quan-ly-model-ai

### Main Success Scenario

- [ ] Ứng dụng gọi API chọn xem danh sách, xem chi tiết, tạo, cập nhật hoặc xóa model.
- [ ] Hệ thống kiểm tra dữ liệu model.
- [ ] Hệ thống thực hiện thao tác.
- [ ] Hệ thống trả model hoặc trạng thái thay đổi.
### Alternative Flows

#### A1: Không tìm thấy model
**Trigger:** Model cần xem hoặc cập nhật không tồn tại (step 2)  
**Flow:**
- [ ] Hệ thống báo không tìm thấy.
- [ ] Use case ends.
#### A2: Trùng tên model
**Trigger:** Tên model mới đã tồn tại (step 2)  
**Flow:**
- [ ] Hệ thống từ chối lưu model trùng.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Danh mục model phản ánh thao tác quản trị đã hoàn tất.
#### Failure Postconditions
- [ ] Không tạo model thiếu tên hoặc địa chỉ dịch vụ.
### Business Rules

#### BR-001: Tên model duy nhất
Tên model là bắt buộc và duy nhất.
#### BR-002: Địa chỉ dịch vụ bắt buộc
Mỗi model phải có địa chỉ dịch vụ để định tuyến tác vụ AI.

## UC-013-in-anh-qua-may-in-dicom

### Main Success Scenario

- [ ] Người dùng Viewer đăng ký hoặc cập nhật máy in với thông tin kết nối hợp lệ.
- [ ] Người dùng Viewer chọn máy in, ảnh và tùy chọn in.
- [ ] Hệ thống kiểm tra máy in và dữ liệu ảnh.
- [ ] Hệ thống tạo tác vụ in và gửi tới máy in DICOM.
- [ ] Máy in DICOM phản hồi kết quả xử lý.
- [ ] Hệ thống trả kết quả tác vụ cho Viewer.
### Alternative Flows

#### A1: Máy in không khả dụng
**Trigger:** Không tìm thấy máy in đã chọn hoặc máy in đang bị vô hiệu hóa (step 3)  
**Flow:**
- [ ] Hệ thống không gửi tác vụ in.
- [ ] Use case ends.
#### A2: Cấu hình máy in không hợp lệ
**Trigger:** Thiếu tên, host, AE Title hoặc cổng ngoài phạm vi (step 1)  
**Flow:**
- [ ] Hệ thống từ chối lưu cấu hình.
- [ ] Use case ends.
#### A3: Không xử lý được tác vụ in
**Trigger:** Không đọc được dữ liệu ảnh hoặc xảy ra lỗi khi gửi tác vụ tới máy in (step 4)  
**Flow:**
- [ ] Hệ thống trả kết quả in không thành công cùng lý do lỗi.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Cấu hình máy in được lưu hoặc tác vụ in đã được gửi và có kết quả.
#### Failure Postconditions
- [ ] Không gửi tác vụ tới máy in không tồn tại hoặc có cấu hình không hợp lệ.
### Business Rules

#### BR-001: Phạm vi cổng
Cổng máy in phải từ 1 đến 65535.
#### BR-002: Thông tin kết nối bắt buộc
Tên máy in, host, AE Title phía gửi và AE Title phía nhận đều bắt buộc, tối đa 255 ký tự.
#### BR-003: Dữ liệu in bắt buộc
Yêu cầu in phải xác định máy in và chứa ảnh.
#### BR-004: Kết quả lỗi in
Ảnh không đọc được hoặc lỗi trong quá trình in được trả thành kết quả không thành công; tác vụ không được báo là đã in thành công.

## UC-014-theo-doi-license-va-nguoi-dung-dong-thoi

### Main Success Scenario

- [ ] Người dùng Viewer cung cấp fingerprint và mở kênh theo dõi hoặc gửi nhịp hoạt động.
- [ ] Hệ thống sử dụng bộ theo dõi riêng tương ứng với kênh sự kiện hoặc heartbeat.
- [ ] Hệ thống kiểm tra trạng thái license và giới hạn người dùng đồng thời trong bộ theo dõi được chọn.
- [ ] Hệ thống ghi nhận hoặc làm mới fingerprint khi license còn hiệu lực và vẫn còn chỗ trống, hoặc khi license không giới hạn người dùng.
- [ ] Hệ thống trả snapshot của bộ theo dõi tương ứng hoặc phát cập nhật trạng thái.
- [ ] Khi người dùng mở Viewer, hệ thống chỉ cung cấp giao diện Viewer nếu license hợp lệ.
- [ ] Bộ lập lịch loại bỏ các heartbeat và kết nối đã hết hiệu lực.
### Alternative Flows

#### A1: Thiếu fingerprint
**Trigger:** Viewer không cung cấp fingerprint hợp lệ (step 1)  
**Flow:**
- [ ] Hệ thống từ chối mở kênh hoặc không ghi nhận heartbeat.
- [ ] Use case ends.
#### A2: License hết hạn trong lúc theo dõi
**Trigger:** Trạng thái license hiệu lực chuyển sang hết hạn (step 3)  
**Flow:**
- [ ] Hệ thống phát trạng thái hết hạn và kết thúc các kết nối theo dõi không còn hợp lệ.
- [ ] Use case ends.
#### A3: Đạt giới hạn người dùng đồng thời
**Trigger:** Một fingerprint mới xuất hiện khi license có giới hạn dương và số người dùng trong bộ theo dõi đã đạt giới hạn (step 4)  
**Flow:**
- [ ] Hệ thống từ chối ghi nhận fingerprint mới và trả trạng thái quá giới hạn.
- [ ] Use case ends.
#### A4: License không hợp lệ khi mở Viewer
**Trigger:** Người dùng mở Viewer khi license không hợp lệ (step 6)  
**Flow:**
- [ ] Hệ thống chuyển người dùng tới giao diện thông báo license không hợp lệ.
- [ ] Use case ends.
### Postconditions

#### Success Postconditions
- [ ] Snapshot license và số người dùng đồng thời phản ánh tập client còn hoạt động.
#### Failure Postconditions
- [ ] Fingerprint trống không được tính là người dùng hoạt động.
### Business Rules

#### BR-001: Fingerprint bắt buộc
Kênh sự kiện và heartbeat chỉ chấp nhận fingerprint khác rỗng.
#### BR-002: Thu hồi hoạt động cũ
Fingerprint heartbeat không được làm mới trong 45 giây sẽ hết hiệu lực; bộ lập lịch dọn heartbeat mỗi 15 giây. Phiên kênh sự kiện không còn kết nối dùng thời hạn cấu hình, mặc định 45 giây; tác vụ keep-alive chạy theo khoảng cấu hình, mặc định 5 giây, đồng thời dò và loại các kết nối không còn hoạt động.
#### BR-003: Một fingerprint chiếm một chỗ
Nhiều kết nối cùng fingerprint chỉ được tính là một người dùng đồng thời.
#### BR-004: Giới hạn người dùng đồng thời
Fingerprint mới không được ghi nhận khi giới hạn license lớn hơn 0 và số người dùng trong bộ theo dõi đã bằng giới hạn đó.
#### BR-005: Quyền mở Viewer theo license
Giao diện Viewer chỉ được cung cấp khi license hiện tại hợp lệ.
#### BR-006: License không giới hạn người dùng
Giới hạn người dùng nhỏ hơn hoặc bằng 0 cho phép ghi nhận không giới hạn fingerprint.
#### BR-007: Hai bộ theo dõi CCU độc lập
Kênh sự kiện và heartbeat duy trì hai tập fingerprint cùng snapshot riêng; số người dùng trong trạng thái license tổng hợp được lấy từ bộ theo dõi kênh sự kiện.

