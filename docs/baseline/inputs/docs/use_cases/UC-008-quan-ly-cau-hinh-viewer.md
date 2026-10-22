# Use Case: Quản lý cấu hình Viewer

## Overview

**Use Case ID:** UC-008  
**Use Case Name:** Quản lý cấu hình Viewer  
**Primary Actor:** Người dùng Viewer, Quản trị viên  
**Goal:** Tra cứu và duy trì các giá trị cấu hình điều khiển hành vi Viewer.  
**Trigger:** Actor yêu cầu xem hoặc thay đổi một cấu hình Viewer.  
**Status:** Draft

## Preconditions

- Người dùng Viewer có phiên hợp lệ; quản trị viên đã được xác thực cho vùng quản trị.

## Main Success Scenario

1. Actor chọn xem danh sách, xem chi tiết, tạo, cập nhật hoặc xóa cấu hình.
2. Hệ thống kiểm tra quyền áp dụng cho thao tác.
3. Hệ thống kiểm tra dữ liệu cấu hình.
4. Hệ thống thực hiện thao tác và trả kết quả.

## Alternative Flows

### A1: Thiếu quyền sửa cấu hình

**Trigger:** Người dùng Viewer không có quyền sửa overlay khi tạo hoặc cập nhật (step 2)  
**Flow:**

1. Hệ thống từ chối thay đổi.
2. Use case ends.

### A2: Không tìm thấy cấu hình

**Trigger:** Cấu hình cần xem hoặc cập nhật không tồn tại (step 3)  
**Flow:**

1. Hệ thống báo không tìm thấy.
2. Use case ends.

## Postconditions

### Success Postconditions

- Cấu hình được đọc hoặc trạng thái lưu trữ phản ánh thay đổi đã yêu cầu.

### Failure Postconditions

- Không áp dụng thay đổi khi actor thiếu quyền hoặc cấu hình đích không tồn tại.

## Business Rules

### BR-001: Thuộc tính bắt buộc

Định danh và giá trị cấu hình là bắt buộc khi tạo cấu hình.

### BR-002: Quyền sửa overlay

Người dùng phiên phải có quyền MODIFY_OVERLAY_CONFIG hoặc quyền ADMIN để tạo hay cập nhật cấu hình qua phạm vi phiên.
