# Use Case: Quản lý phiên dựng hình 3D

## Overview

**Use Case ID:** UC-007  
**Use Case Name:** Quản lý phiên dựng hình 3D  
**Primary Actor:** Người dùng Viewer, Ứng dụng gọi API  
**Secondary Actors:** Bộ lập lịch  
**Goal:** Khởi tạo, sử dụng, kiểm tra và giải phóng phiên cùng tài nguyên dựng hình 3D.  
**Trigger:** Người dùng Viewer yêu cầu liên kết dựng hình hoặc ứng dụng gọi API yêu cầu quản lý phiên hay tài nguyên 3D.  
**Status:** Draft

## Preconditions

_None — không có điều kiện chung áp dụng cho mọi thao tác; luồng tạo và kết nối tự kiểm tra phiên liên quan._

## Main Success Scenario

1. Người dùng Viewer cung cấp phiên 2D và thông tin dữ liệu cần dựng hình.
2. Hệ thống cấp một tiến trình dựng hình và tạo phiên 3D.
3. Hệ thống trả liên kết kết nối thời gian thực.
4. Người dùng Viewer mở kết nối bằng định danh phiên 2D và phiên 3D.
5. Hệ thống kiểm tra hai phiên rồi chuyển tiếp thông điệp giữa Viewer và tiến trình dựng hình.
6. Khi kết nối kết thúc, hệ thống giải phóng phiên và tài nguyên theo trạng thái sử dụng.

## Alternative Flows

### A1: Thiếu tài nguyên 3D

**Trigger:** Không còn bộ nhớ, cổng hoặc tiến trình khả dụng (step 2)  
**Flow:**

1. Hệ thống không tạo phiên 3D và báo dịch vụ tạm không sẵn sàng.
2. Use case ends.

### A2: Phiên 2D không hợp lệ khi tạo phiên 3D

**Trigger:** Không tìm thấy phiên 2D nguồn (step 2)  
**Flow:**

1. Hệ thống không tạo phiên 3D và từ chối yêu cầu.
2. Use case ends.

### A3: Phiên không hợp lệ khi kết nối

**Trigger:** Thiếu hoặc không tìm thấy phiên 2D hay phiên 3D (step 5)  
**Flow:**

1. Hệ thống đóng kết nối.
2. Use case ends.

### A4: Tra cứu trạng thái phiên hoặc tài nguyên

**Trigger:** Ứng dụng gọi API yêu cầu danh sách phiên, chi tiết phiên hoặc các cổng khả dụng (step 1)  
**Flow:**

1. Hệ thống trả danh sách phiên, phiên theo định danh hoặc danh sách cổng khả dụng tương ứng.
2. Use case ends.

### A5: Chủ động xóa phiên

**Trigger:** Ứng dụng gọi API yêu cầu xóa một phiên (step 1)  
**Flow:**

1. Nếu phiên tồn tại, hệ thống trả tiến trình về nhóm tài nguyên khi có thể và xóa phiên khỏi registry.
2. Hệ thống trả kết quả thành công cho định danh được yêu cầu.
3. Use case ends.

### A6: Đặt lại registry tài nguyên

**Trigger:** Ứng dụng gọi API yêu cầu buộc đặt lại tài nguyên (step 1)  
**Flow:**

1. Hệ thống xóa registry phiên và khởi tạo lại danh sách cổng khả dụng.
2. Hệ thống trả danh sách cổng khả dụng sau thao tác.
3. Use case ends.

### A7: Định danh phiên trống

**Trigger:** Định danh phiên để tra cứu hoặc xóa bị trống (step 1)  
**Flow:**

1. Hệ thống từ chối thao tác quản lý phiên.
2. Use case ends.

### A8: Tra cứu phiên không tồn tại

**Trigger:** Không tìm thấy phiên được yêu cầu khi tra cứu chi tiết (step 1)  
**Flow:**

1. Hệ thống từ chối yêu cầu tra cứu chi tiết.
2. Use case ends.

### A9: Xóa phiên không tồn tại

**Trigger:** Không tìm thấy phiên được yêu cầu khi xóa (step 1)  
**Flow:**

1. Hệ thống không thay đổi registry và vẫn trả kết quả thành công cho định danh được yêu cầu.
2. Use case ends.

## Postconditions

### Success Postconditions

- Phiên 3D hoạt động trong thời gian kết nối và tài nguyên được trả lại sau khi kết thúc.

### Failure Postconditions

- Không giữ lại tài nguyên cho phiên 3D không được thiết lập thành công.

## Business Rules

### BR-001: Ghép cặp phiên

Kết nối dựng hình chỉ được thiết lập khi cả phiên 2D và phiên 3D được xác định và tồn tại.

### BR-002: Thu hồi tiến trình

Tiến trình đã kết thúc được kiểm tra định kỳ và thu hồi khỏi nhóm tài nguyên.

### BR-003: Chu kỳ thu hồi tiến trình

Bộ lập lịch kiểm tra và thu hồi tiến trình đã kết thúc mỗi phút.

### BR-004: Quản lý tài nguyên qua API

Ứng dụng gọi API có thể xem phiên 3D, xem cổng khả dụng, xóa một phiên hoặc đặt lại registry phiên và danh sách cổng. Thao tác đặt lại registry không trực tiếp dừng các tiến trình 3D đang chạy.
