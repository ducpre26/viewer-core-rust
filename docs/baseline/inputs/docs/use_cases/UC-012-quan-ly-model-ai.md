# Use Case: Quản lý model AI

## Overview

**Use Case ID:** UC-012  
**Use Case Name:** Quản lý model AI  
**Primary Actor:** Ứng dụng gọi API  
**Goal:** Đăng ký và duy trì danh mục model dùng cho các tác vụ AI.  
**Trigger:** Ứng dụng gọi API yêu cầu xem hoặc thay đổi một model AI.  
**Status:** Draft

## Preconditions

- Không có điều kiện xác thực hoặc phân quyền được áp dụng riêng cho chức năng quản lý model.

## Main Success Scenario

1. Ứng dụng gọi API chọn xem danh sách, xem chi tiết, tạo, cập nhật hoặc xóa model.
2. Hệ thống kiểm tra dữ liệu model.
3. Hệ thống thực hiện thao tác.
4. Hệ thống trả model hoặc trạng thái thay đổi.

## Alternative Flows

### A1: Không tìm thấy model

**Trigger:** Model cần xem hoặc cập nhật không tồn tại (step 2)  
**Flow:**

1. Hệ thống báo không tìm thấy.
2. Use case ends.

### A2: Trùng tên model

**Trigger:** Tên model mới đã tồn tại (step 2)  
**Flow:**

1. Hệ thống từ chối lưu model trùng.
2. Use case ends.

## Postconditions

### Success Postconditions

- Danh mục model phản ánh thao tác quản trị đã hoàn tất.

### Failure Postconditions

- Không tạo model thiếu tên hoặc địa chỉ dịch vụ.

## Business Rules

### BR-001: Tên model duy nhất

Tên model là bắt buộc và duy nhất.

### BR-002: Địa chỉ dịch vụ bắt buộc

Mỗi model phải có địa chỉ dịch vụ để định tuyến tác vụ AI.
