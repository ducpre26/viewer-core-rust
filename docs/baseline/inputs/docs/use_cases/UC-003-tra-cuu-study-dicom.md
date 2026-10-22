# Use Case: Tra cứu study DICOM

## Overview

**Use Case ID:** UC-003  
**Use Case Name:** Tra cứu study DICOM  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Kho DICOM  
**Goal:** Tìm các study DICOM phù hợp với bộ lọc để lựa chọn xem.  
**Trigger:** Người dùng Viewer gửi tiêu chí tra cứu study.  
**Status:** Draft

## Preconditions

- Phiên 2D tồn tại và chứa thông tin kết nối tới kho DICOM.

## Main Success Scenario

1. Người dùng Viewer cung cấp định danh phiên, Patient ID, các tiêu chí tùy chọn và thông tin phân trang.
2. Hệ thống kiểm tra phiên và chuẩn hóa vị trí bắt đầu cùng giới hạn kết quả.
3. Hệ thống chuyển tiêu chí tra cứu tới kho DICOM.
4. Kho DICOM trả các study phù hợp.
5. Hệ thống chuẩn hóa kết quả và trả danh sách study.

## Alternative Flows

### A1: Không có kết quả

**Trigger:** Kho DICOM không trả study phù hợp (step 4)  
**Flow:**

1. Hệ thống trả trạng thái không có nội dung.
2. Use case ends.

### A2: Phiên không tồn tại

**Trigger:** Không tìm thấy phiên được cung cấp (step 2)  
**Flow:**

1. Hệ thống trả trạng thái không có nội dung.
2. Use case ends.

### A3: Thiếu dữ liệu tra cứu bắt buộc

**Trigger:** Thiếu định danh phiên hoặc Patient ID (step 1)  
**Flow:**

1. Hệ thống từ chối yêu cầu tra cứu.
2. Use case ends.

## Postconditions

### Success Postconditions

- Người dùng nhận được danh sách study phù hợp; dữ liệu nghiệp vụ không bị thay đổi.

### Failure Postconditions

- Không có study hoặc phiên nào được tạo hay sửa.

## Business Rules

### BR-001: Giới hạn mặc định

Khi actor không chỉ định, kết quả bắt đầu từ vị trí đầu tiên và giới hạn mặc định là 100 study. Vị trí âm được chuẩn hóa về 0; giới hạn không dương được chuẩn hóa thành 20.

### BR-002: Dữ liệu tra cứu bắt buộc

Định danh phiên phải khác rỗng và Patient ID là bắt buộc.

### BR-003: Tham số sắp xếp chưa được áp dụng

Giá trị sắp xếp có thể được tiếp nhận nhưng không được dùng để thay đổi truy vấn hoặc thứ tự kết quả.
