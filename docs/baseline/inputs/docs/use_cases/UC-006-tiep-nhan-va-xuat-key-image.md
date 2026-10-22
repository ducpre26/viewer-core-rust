# Use Case: Tiếp nhận và xuất key image

## Overview

**Use Case ID:** UC-006  
**Use Case Name:** Tiếp nhận và xuất key image  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Bộ lập lịch, Kho DICOM  
**Goal:** Chuyển key image từ Viewer thành đối tượng DICOM hợp lệ và gửi tới kho DICOM đích.  
**Trigger:** Người dùng Viewer gửi một lô key image của phiên.  
**Status:** Draft

## Preconditions

- Phiên 2D hợp lệ tồn tại.

## Main Success Scenario

1. Người dùng Viewer gửi metadata DICOM và dữ liệu ảnh của key image.
2. Hệ thống kiểm tra cấu trúc lô và các thuộc tính DICOM bắt buộc.
3. Hệ thống bổ sung các thuộc tính có thể suy ra và lưu tạm các đối tượng DICOM.
4. Hệ thống ghi nhận lô đang chờ xuất.
5. Bộ lập lịch chọn lô chưa thành công còn trong giới hạn thử lại.
6. Hệ thống gửi lô tới kho DICOM đích.
7. Khi kho chấp nhận, hệ thống xóa bản ghi chờ và dữ liệu tạm.

## Alternative Flows

### A1: Nội dung gửi lên không hợp lệ

**Trigger:** Thiếu ranh giới lô, thuộc tính DICOM hoặc dữ liệu ảnh bắt buộc (step 2)  
**Flow:**

1. Hệ thống từ chối lô và dọn thư mục tạm của yêu cầu.
2. Use case ends.

### A2: Kho DICOM từ chối lô

**Trigger:** Kho DICOM không chấp nhận lần gửi (step 6)  
**Flow:**

1. Hệ thống tăng số lần thử lại và lưu lý do lỗi.
2. Use case ends.

### A3: Vượt giới hạn key image của study

**Trigger:** Tổng số key image hiện có và mới gửi vượt giới hạn cấu hình của study (step 2)  
**Flow:**

1. Hệ thống từ chối lô key image mới.
2. Hệ thống dọn dữ liệu tạm của yêu cầu.
3. Use case ends.

## Postconditions

### Success Postconditions

- Key image đã được kho DICOM chấp nhận và dữ liệu tạm tương ứng đã được dọn.

### Failure Postconditions

- Dữ liệu tạm của một yêu cầu tiếp nhận thất bại được dọn; lô xuất thất bại còn được theo dõi nếu chưa hết số lần thử.

## Business Rules

### BR-001: Giới hạn thử lại

Bộ lập lịch chỉ chọn lô chưa thành công có số lần thử lại nhỏ hơn hoặc bằng 3.

### BR-002: Xuất theo cấu hình

Việc gửi lô chờ tới kho DICOM chỉ chạy khi tùy chọn gửi key image được bật.

### BR-003: Giới hạn key image theo study

Tổng số key image đã ghi nhận và mới tiếp nhận của một study không được vượt giới hạn cấu hình của hệ thống.

### BR-004: Giới hạn lô gửi mỗi lượt

Mỗi lượt chạy, Bộ lập lịch xử lý tối đa 100 lô key image đang chờ gửi.
