# Use Case: Phân tích bất thường bằng AI

## Overview

**Use Case ID:** UC-010  
**Use Case Name:** Phân tích bất thường bằng AI  
**Primary Actor:** Người dùng Viewer  
**Secondary Actors:** Máy chủ AI  
**Goal:** Nhận kết quả phân tích bất thường X-quang từ model AI đã đăng ký.  
**Trigger:** Người dùng Viewer gửi yêu cầu phân tích và chọn model.  
**Status:** Draft

## Preconditions

- Model AI được chọn đã được đăng ký và có địa chỉ dịch vụ.

## Main Success Scenario

1. Người dùng Viewer cung cấp dữ liệu ca chụp và tên model.
2. Hệ thống tìm cấu hình model tương ứng.
3. Hệ thống gửi yêu cầu phân tích tới máy chủ AI.
4. Máy chủ AI trả kết quả study và series.
5. Hệ thống chuẩn hóa kết quả thành trạng thái thành công và trả về Viewer.

## Alternative Flows

### A1: Thiếu hoặc không tìm thấy model

**Trigger:** Tên model trống hoặc không có model tương ứng (step 2)  
**Flow:**

1. Hệ thống từ chối yêu cầu phân tích.
2. Use case ends.

### A2: Máy chủ AI không trả kết quả

**Trigger:** Kết quả từ máy chủ AI rỗng (step 4)  
**Flow:**

1. Hệ thống báo suy luận thất bại.
2. Use case ends.

## Postconditions

### Success Postconditions

- Viewer nhận kết quả phân tích gắn với model được chọn.

### Failure Postconditions

- Không có model hay dữ liệu nghiệp vụ bền vững bị thay đổi.

## Business Rules

### BR-001: Chọn model bắt buộc

Tên model không được để trống và phải khớp một model đã đăng ký.
