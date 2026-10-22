# Use Case: In ảnh qua máy in DICOM

## Overview

**Use Case ID:** UC-013  
**Use Case Name:** In ảnh qua máy in DICOM  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Máy in DICOM  
**Goal:** Duy trì máy in và gửi ảnh được chọn thành tác vụ in DICOM.  
**Trigger:** Actor yêu cầu quản lý máy in hoặc in một ảnh từ phiên Viewer.  
**Status:** Draft

## Preconditions

- Phiên 2D hợp lệ tồn tại đối với thao tác in hoặc quản lý máy in.

## Main Success Scenario

1. Người dùng Viewer đăng ký hoặc cập nhật máy in với thông tin kết nối hợp lệ.
2. Người dùng Viewer chọn máy in, ảnh và tùy chọn in.
3. Hệ thống kiểm tra máy in và dữ liệu ảnh.
4. Hệ thống tạo tác vụ in và gửi tới máy in DICOM.
5. Máy in DICOM phản hồi kết quả xử lý.
6. Hệ thống trả kết quả tác vụ cho Viewer.

## Alternative Flows

### A1: Máy in không khả dụng

**Trigger:** Không tìm thấy máy in đã chọn hoặc máy in đang bị vô hiệu hóa (step 3)  
**Flow:**

1. Hệ thống không gửi tác vụ in.
2. Use case ends.

### A2: Cấu hình máy in không hợp lệ

**Trigger:** Thiếu tên, host, AE Title hoặc cổng ngoài phạm vi (step 1)  
**Flow:**

1. Hệ thống từ chối lưu cấu hình.
2. Use case ends.

### A3: Không xử lý được tác vụ in

**Trigger:** Không đọc được dữ liệu ảnh hoặc xảy ra lỗi khi gửi tác vụ tới máy in (step 4)  
**Flow:**

1. Hệ thống trả kết quả in không thành công cùng lý do lỗi.
2. Use case ends.

## Postconditions

### Success Postconditions

- Cấu hình máy in được lưu hoặc tác vụ in đã được gửi và có kết quả.

### Failure Postconditions

- Không gửi tác vụ tới máy in không tồn tại hoặc có cấu hình không hợp lệ.

## Business Rules

### BR-001: Phạm vi cổng

Cổng máy in phải từ 1 đến 65535.

### BR-002: Thông tin kết nối bắt buộc

Tên máy in, host, AE Title phía gửi và AE Title phía nhận đều bắt buộc, tối đa 255 ký tự.

### BR-003: Dữ liệu in bắt buộc

Yêu cầu in phải xác định máy in và chứa ảnh.

### BR-004: Kết quả lỗi in

Ảnh không đọc được hoặc lỗi trong quá trình in được trả thành kết quả không thành công; tác vụ không được báo là đã in thành công.
