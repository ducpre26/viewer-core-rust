# Use Case: Tải nội dung DICOM

## Overview

**Use Case ID:** UC-005  
**Use Case Name:** Tải nội dung DICOM  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Kho DICOM  
**Goal:** Tải study, series hoặc instance dưới dạng tệp truyền trực tiếp hay gói nén.  
**Trigger:** Người dùng Viewer yêu cầu tải một đối tượng DICOM trong phiên.  
**Status:** Draft

## Preconditions

- Phiên 2D hợp lệ và có thể truy cập kho DICOM.

## Main Success Scenario

1. Người dùng Viewer chọn study, series hoặc instance và kiểu tải.
2. Hệ thống lấy các instance tương ứng từ kho DICOM.
3. Hệ thống đóng gói hoặc truyền dữ liệu theo kiểu tải đã chọn.
4. Hệ thống phát luồng tệp về cho người dùng.

## Alternative Flows

### A1: Kiểu tải không được nhận diện

**Trigger:** Không cung cấp kiểu tải hoặc giá trị kiểu tải không thể được diễn giải (step 1)  
**Flow:**

1. Hệ thống diễn giải yêu cầu thành kiểu luồng nhị phân mặc định.
2. Hệ thống từ chối kiểu này vì không thuộc tập kiểu được hỗ trợ tại phạm vi tải study, series hoặc instance.
3. Use case ends.

### A2: Không lấy được dữ liệu nguồn

**Trigger:** Kho DICOM không trả được dữ liệu yêu cầu (step 2)  
**Flow:**

1. Hệ thống kết thúc luồng tải với lỗi.
2. Use case ends.

### A3: Study không thuộc phiên

**Trigger:** Study được chọn không nằm trong danh sách study của phiên (step 2)  
**Flow:**

1. Hệ thống không cung cấp nội dung DICOM.
2. Use case ends.

## Postconditions

### Success Postconditions

- Người dùng nhận được nội dung DICOM ở phạm vi và định dạng đã chọn.

### Failure Postconditions

- Không có thay đổi đối với phiên hoặc kho DICOM.

## Business Rules

### BR-001: Kiểu tải mặc định

Khi không chỉ định hoặc không nhận diện được kiểu tải, hệ thống chuyển giá trị thành dạng luồng nhị phân; các thao tác tải study, series và instance hiện tại đều từ chối dạng này.

### BR-002: Phạm vi study của phiên

Chỉ được tải study có trong danh sách study của phiên đang sử dụng.

### BR-003: Kiểu tải theo phạm vi

Tải study và instance chỉ chấp nhận DICOM hoặc JPEG; tải series chấp nhận DICOM, JPEG hoặc MP4.
