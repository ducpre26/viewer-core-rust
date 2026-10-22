# Use Case: Quản lý và chia sẻ phiên 2D

## Overview

**Use Case ID:** UC-002  
**Use Case Name:** Quản lý và chia sẻ phiên 2D  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Bộ lập lịch  
**Goal:** Xem, cập nhật, chia sẻ hoặc kết thúc một phiên xem ảnh đang tồn tại.  
**Trigger:** Người dùng Viewer thao tác với phiên hiện tại hoặc yêu cầu tạo liên kết chia sẻ.  
**Status:** Draft

## Preconditions

- Phiên 2D tồn tại và chưa bị từ chối bởi kiểm tra truy cập phiên.

## Main Success Scenario

1. Người dùng Viewer chọn thao tác xem, cập nhật, chia sẻ hoặc kết thúc phiên.
2. Hệ thống kiểm tra phiên và khóa chia sẻ nếu phiên yêu cầu khóa.
3. Khi cập nhật, hệ thống thay đổi danh sách study của phiên ngắn hạn.
4. Khi chia sẻ, hệ thống xác nhận phiên nguồn không phải là phiên chia sẻ và xác định thời hạn hiệu lực.
5. Hệ thống tạo một phiên chia sẻ dài hạn với tùy chọn ẩn danh và khóa truy cập đã chọn.
6. Khi kết thúc phiên, hệ thống xóa phiên khỏi cả vùng lưu trữ ngắn hạn và dài hạn.
7. Hệ thống trả thông tin phiên hoặc liên kết chia sẻ tương ứng.

## Alternative Flows

### A1: Phiên hoặc khóa chia sẻ không hợp lệ

**Trigger:** Phiên không tồn tại hoặc khóa cung cấp không khớp (step 2)  
**Flow:**

1. Hệ thống từ chối truy cập phiên.
2. Use case ends.

### A2: Phiên cập nhật không phải phiên ngắn hạn

**Trigger:** Không tìm thấy phiên trong vùng lưu trữ ngắn hạn khi cập nhật (step 3)  
**Flow:**

1. Hệ thống không cập nhật danh sách study.
2. Use case ends.

### A3: Chia sẻ lại phiên chia sẻ

**Trigger:** Phiên nguồn đã là một phiên chia sẻ (step 4)  
**Flow:**

1. Hệ thống không tạo thêm phiên chia sẻ.
2. Use case ends.

### A4: Thời hạn chia sẻ ngoài phạm vi

**Trigger:** Thời hạn chia sẻ không dương hoặc vượt giới hạn cấu hình (step 4)  
**Flow:**

1. Hệ thống đặt thời hạn phiên chia sẻ thành một giờ kể từ thời điểm tạo.
2. Use case continues at step 5.

### A5: Không tạo được phiên chia sẻ

**Trigger:** Dịch vụ phiên không thể lưu phiên chia sẻ (step 5)  
**Flow:**

1. Hệ thống không cấp liên kết chia sẻ.
2. Use case ends.

## Postconditions

### Success Postconditions

- Phiên được đọc hoặc thay đổi theo yêu cầu; nếu chia sẻ thành công, một phiên chia sẻ riêng tồn tại.

### Failure Postconditions

- Không cấp quyền truy cập cho phiên hoặc khóa chia sẻ không hợp lệ.

## Business Rules

### BR-001: Khóa phiên chia sẻ

Phiên có share key chỉ chấp nhận truy cập khi giá trị được cung cấp khớp với share key của phiên.

### BR-002: Thời hạn chia sẻ bắt buộc

Yêu cầu chia sẻ phải cung cấp thời lượng hết hạn.

### BR-003: Phạm vi cập nhật phiên

Thao tác cập nhật chỉ thay đổi danh sách study và chỉ áp dụng cho phiên đang có trong vùng lưu trữ ngắn hạn.

### BR-004: Không chia sẻ lại

Một phiên đã là phiên chia sẻ không thể được dùng để tạo thêm phiên chia sẻ.

### BR-005: Chuẩn hóa thời hạn chia sẻ

Thời hạn không dương hoặc vượt giới hạn cấu hình được thay bằng một giờ.

### BR-006: Định dạng khóa chia sẻ

Khi được yêu cầu, khóa của phiên chia sẻ gồm sáu chữ số.

### BR-007: Dọn phiên dài hạn hết hạn

Bộ lập lịch xóa các phiên dài hạn đã hết hạn vào nửa đêm mỗi ngày theo múi giờ Asia/Ho_Chi_Minh.
