# Use Case: Khởi tạo phiên xem ảnh 2D

## Overview

**Use Case ID:** UC-001  
**Use Case Name:** Khởi tạo phiên xem ảnh 2D  
**Primary Actor:** Hệ thống client PACS  
**Goal:** Tạo một phiên có thời hạn và nhận liên kết để mở các study đã chọn trong Viewer.  
**Trigger:** Hệ thống client PACS yêu cầu một liên kết Viewer cho các study.  
**Status:** Draft

## Preconditions

- Client đã được đăng ký hoặc cơ chế kiểm tra client được vô hiệu hóa bằng cấu hình vận hành.

## Main Success Scenario

1. Hệ thống client PACS cung cấp kho DICOM, danh sách study, người dùng và thời hạn mong muốn.
2. Hệ thống xác thực client và kiểm tra giới hạn địa chỉ mạng nếu được cấu hình.
3. Hệ thống lấy các quyền đã gán cho client.
4. Hệ thống chuẩn hóa thời hạn và tạo phiên chứa các study cùng quyền hiệu lực.
5. Hệ thống trả về liên kết Viewer của phiên.

## Alternative Flows

### A1: Client không hợp lệ

**Trigger:** Không thể xác thực client hoặc địa chỉ mạng không được phép (step 2)  
**Flow:**

1. Hệ thống từ chối yêu cầu tạo phiên.
2. Use case ends.

### A2: Dữ liệu phiên thiếu

**Trigger:** Thiếu kho DICOM hoặc danh sách study (step 1)  
**Flow:**

1. Hệ thống báo dữ liệu không hợp lệ.
2. Use case ends.

## Postconditions

### Success Postconditions

- Phiên 2D có thời hạn tồn tại trong vùng lưu trữ phiên và liên kết Viewer được cấp.

### Failure Postconditions

- Không có phiên mới được tạo từ yêu cầu không hợp lệ.

## Business Rules

### BR-001: Dữ liệu bắt buộc

Kho DICOM và tập study là bắt buộc khi tạo phiên.

### BR-002: Giới hạn thời hạn

Thời hạn không dương hoặc vượt giới hạn cấu hình được thay bằng thời hạn mặc định của hệ thống.

### BR-003: Quyền theo client

Quyền của phiên được chụp từ các quyền hiện được gán cho client tại thời điểm tạo phiên.
