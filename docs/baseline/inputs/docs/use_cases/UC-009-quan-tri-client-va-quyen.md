# Use Case: Quản trị client và quyền

## Overview

**Use Case ID:** UC-009  
**Use Case Name:** Quản trị client và quyền  
**Primary Actor:** Quản trị viên  
**Goal:** Duy trì client, danh mục quyền và các quyền được gán cho từng client.  
**Trigger:** Quản trị viên yêu cầu xem hoặc thay đổi client hay quyền.  
**Status:** Draft

## Preconditions

- Quản trị viên đã xác thực với vai trò quản trị.

## Main Success Scenario

1. Quản trị viên chọn tài nguyên client, quyền hoặc ánh xạ client–quyền.
2. Quản trị viên chọn xem, tạo, cập nhật, xóa hoặc gán/bỏ gán.
3. Hệ thống kiểm tra sự tồn tại và dữ liệu bắt buộc.
4. Hệ thống thực hiện thao tác.
5. Hệ thống trả trạng thái hoặc dữ liệu quản trị mới nhất.

## Alternative Flows

### A1: Client hoặc quyền không tồn tại

**Trigger:** Tài nguyên tham chiếu không tồn tại (step 3)  
**Flow:**

1. Hệ thống không tạo hay xóa ánh xạ.
2. Use case ends.

### A2: Dữ liệu tạo không hợp lệ

**Trigger:** Thiếu định danh, bí mật client, tên quyền hoặc định danh quyền (step 3)  
**Flow:**

1. Hệ thống từ chối thay đổi.
2. Use case ends.

## Postconditions

### Success Postconditions

- Client, quyền hoặc ánh xạ được lưu đúng theo thao tác quản trị.
- Khi client hoặc quyền bị xóa, mọi ánh xạ client–quyền tham chiếu tới đối tượng đó cũng bị xóa.

### Failure Postconditions

- Không tạo ánh xạ mồ côi tới client hoặc quyền không tồn tại.

## Business Rules

### BR-001: Truy cập quản trị

Chỉ principal mang vai trò quản trị mới được thực hiện các thao tác của vùng quản trị.

### BR-002: Tính duy nhất của quyền

Tên quyền phải duy nhất trong danh mục quyền.

### BR-003: Ánh xạ không trùng

Một cặp client và quyền chỉ xuất hiện tối đa một lần.

### BR-004: Xóa lan truyền ánh xạ quyền

Xóa client hoặc quyền đồng thời xóa mọi ánh xạ client–quyền liên quan.
