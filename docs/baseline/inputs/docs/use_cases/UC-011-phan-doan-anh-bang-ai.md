# Use Case: Phân đoạn ảnh bằng AI

## Overview

**Use Case ID:** UC-011  
**Use Case Name:** Phân đoạn ảnh bằng AI  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Máy chủ AI  
**Goal:** Tạo tác vụ phân đoạn, theo dõi, nhận thống kê và lưu kết quả để dựng hình.  
**Trigger:** Người dùng Viewer yêu cầu phân đoạn dữ liệu của một phiên.  
**Status:** Draft

## Preconditions

- Model phân đoạn đã được đăng ký.
- Phiên 2D nguồn tồn tại.

## Main Success Scenario

1. Người dùng Viewer cung cấp phiên, model và dữ liệu study/series cần phân đoạn.
2. Hệ thống lấy thông tin kho DICOM từ phiên và tạo tác vụ tại máy chủ AI.
3. Người dùng Viewer yêu cầu trạng thái hoặc thống kê bằng định danh tác vụ.
4. Hệ thống lấy trạng thái hoặc thống kê từ máy chủ AI.
5. Khi tác vụ hoàn tất, người dùng yêu cầu kết quả.
6. Hệ thống tải kết quả và lưu vào vùng dữ liệu study/series dùng cho dựng hình.
7. Hệ thống trả vị trí kết quả đã lưu.

## Alternative Flows

### A1: Phiên hoặc model không hợp lệ

**Trigger:** Không tìm thấy phiên hoặc model được chọn (step 2)  
**Flow:**

1. Hệ thống không tạo tác vụ phân đoạn.
2. Use case ends.

### A2: Kết quả không sẵn sàng

**Trigger:** Máy chủ AI trả lỗi hoặc nội dung kết quả rỗng (step 6)  
**Flow:**

1. Hệ thống không tạo tệp kết quả mới.
2. Use case ends.

### A3: Kết quả đã tồn tại

**Trigger:** Tệp kết quả của tác vụ đã có tại vị trí đích (step 6)  
**Flow:**

1. Hệ thống bỏ qua tải lại và dùng vị trí hiện có.
2. Use case continues at step 7.

## Postconditions

### Success Postconditions

- Kết quả phân đoạn tồn tại trong vùng dữ liệu của đúng study và series.

### Failure Postconditions

- Không tạo tệp kết quả một phần khi máy chủ AI không cung cấp nội dung hợp lệ.

## Business Rules

### BR-001: Định danh dữ liệu bắt buộc

Study Instance UID và Series Instance UID phải có trước khi lưu kết quả phân đoạn.

### BR-002: Không tải trùng

Kết quả hiện có của cùng tác vụ không bị ghi đè.
