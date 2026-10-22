# Use Case: Truy xuất dữ liệu ảnh DICOM

## Overview

**Use Case ID:** UC-004  
**Use Case Name:** Truy xuất dữ liệu ảnh DICOM  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Kho DICOM  
**Goal:** Nhận dữ liệu, metadata, thumbnail, frame hoặc bulk data cần để hiển thị ảnh DICOM.  
**Trigger:** Người dùng Viewer yêu cầu một phần dữ liệu của study, series hoặc instance trong phiên.  
**Status:** Draft

## Preconditions

- Phiên 2D hợp lệ và chứa thông tin kết nối kho DICOM.

## Main Success Scenario

1. Người dùng Viewer xác định study và phần dữ liệu cần truy xuất.
2. Hệ thống kiểm tra quyền truy cập phiên.
3. Hệ thống chuyển yêu cầu cùng thông tin phiên tới kho DICOM.
4. Kho DICOM trả dữ liệu và thông tin mô tả phản hồi.
5. Hệ thống truyền dữ liệu phù hợp về Viewer.

## Alternative Flows

### A1: Phiên hoặc study không thuộc phạm vi truy cập

**Trigger:** Không tìm thấy phiên hoặc study không nằm trong danh sách study của phiên (step 2)  
**Flow:**

1. Hệ thống từ chối truy xuất.
2. Use case ends.

### A2: Kho DICOM từ chối hoặc không có dữ liệu

**Trigger:** Kho DICOM không xác thực yêu cầu hoặc không tìm thấy đối tượng (step 4)  
**Flow:**

1. Hệ thống trả trạng thái lỗi tương ứng và không tạo dữ liệu thay thế.
2. Use case ends.

## Postconditions

### Success Postconditions

- Dữ liệu DICOM yêu cầu được truyền về Viewer mà không thay đổi kho nguồn.

### Failure Postconditions

- Không có dữ liệu DICOM nghiệp vụ nào được ghi bởi hệ thống.

## Business Rules

### BR-001: Phạm vi truy xuất

Hệ thống hỗ trợ dữ liệu ở mức study, series, instance, frame, thumbnail, metadata và bulk data khi kho nguồn cung cấp.

### BR-002: Phạm vi study của phiên

Chỉ được truy xuất study có trong danh sách study của phiên đang sử dụng.
