# Use Case: Theo dõi license và người dùng đồng thời

## Overview

**Use Case ID:** UC-014  
**Use Case Name:** Theo dõi license và người dùng đồng thời  
**Primary Actor:** Người dùng Viewer, Quản trị viên  
**Secondary Actors:** Bộ lập lịch  
**Goal:** Theo dõi trạng thái license và số Viewer đang hoạt động để áp dụng giới hạn sử dụng đồng thời.  
**Trigger:** Viewer mở kênh theo dõi hoặc gửi nhịp hoạt động; quản trị viên yêu cầu trạng thái.  
**Status:** Draft

## Preconditions

- Dịch vụ license đã khởi tạo trạng thái hiệu lực từ nguồn license cấu hình.

## Main Success Scenario

1. Người dùng Viewer cung cấp fingerprint và mở kênh theo dõi hoặc gửi nhịp hoạt động.
2. Hệ thống sử dụng bộ theo dõi riêng tương ứng với kênh sự kiện hoặc heartbeat.
3. Hệ thống kiểm tra trạng thái license và giới hạn người dùng đồng thời trong bộ theo dõi được chọn.
4. Hệ thống ghi nhận hoặc làm mới fingerprint khi license còn hiệu lực và vẫn còn chỗ trống, hoặc khi license không giới hạn người dùng.
5. Hệ thống trả snapshot của bộ theo dõi tương ứng hoặc phát cập nhật trạng thái.
6. Khi người dùng mở Viewer, hệ thống chỉ cung cấp giao diện Viewer nếu license hợp lệ.
7. Bộ lập lịch loại bỏ các heartbeat và kết nối đã hết hiệu lực.

## Alternative Flows

### A1: Thiếu fingerprint

**Trigger:** Viewer không cung cấp fingerprint hợp lệ (step 1)  
**Flow:**

1. Hệ thống từ chối mở kênh hoặc không ghi nhận heartbeat.
2. Use case ends.

### A2: License hết hạn trong lúc theo dõi

**Trigger:** Trạng thái license hiệu lực chuyển sang hết hạn (step 3)  
**Flow:**

1. Hệ thống phát trạng thái hết hạn và kết thúc các kết nối theo dõi không còn hợp lệ.
2. Use case ends.

### A3: Đạt giới hạn người dùng đồng thời

**Trigger:** Một fingerprint mới xuất hiện khi license có giới hạn dương và số người dùng trong bộ theo dõi đã đạt giới hạn (step 4)  
**Flow:**

1. Hệ thống từ chối ghi nhận fingerprint mới và trả trạng thái quá giới hạn.
2. Use case ends.

### A4: License không hợp lệ khi mở Viewer

**Trigger:** Người dùng mở Viewer khi license không hợp lệ (step 6)  
**Flow:**

1. Hệ thống chuyển người dùng tới giao diện thông báo license không hợp lệ.
2. Use case ends.

## Postconditions

### Success Postconditions

- Snapshot license và số người dùng đồng thời phản ánh tập client còn hoạt động.

### Failure Postconditions

- Fingerprint trống không được tính là người dùng hoạt động.

## Business Rules

### BR-001: Fingerprint bắt buộc

Kênh sự kiện và heartbeat chỉ chấp nhận fingerprint khác rỗng.

### BR-002: Thu hồi hoạt động cũ

Fingerprint heartbeat không được làm mới trong 45 giây sẽ hết hiệu lực; bộ lập lịch dọn heartbeat mỗi 15 giây. Phiên kênh sự kiện không còn kết nối dùng thời hạn cấu hình, mặc định 45 giây; tác vụ keep-alive chạy theo khoảng cấu hình, mặc định 5 giây, đồng thời dò và loại các kết nối không còn hoạt động.

### BR-003: Một fingerprint chiếm một chỗ

Nhiều kết nối cùng fingerprint chỉ được tính là một người dùng đồng thời.

### BR-004: Giới hạn người dùng đồng thời

Fingerprint mới không được ghi nhận khi giới hạn license lớn hơn 0 và số người dùng trong bộ theo dõi đã bằng giới hạn đó.

### BR-005: Quyền mở Viewer theo license

Giao diện Viewer chỉ được cung cấp khi license hiện tại hợp lệ.

### BR-006: License không giới hạn người dùng

Giới hạn người dùng nhỏ hơn hoặc bằng 0 cho phép ghi nhận không giới hạn fingerprint.

### BR-007: Hai bộ theo dõi CCU độc lập

Kênh sự kiện và heartbeat duy trì hai tập fingerprint cùng snapshot riêng; số người dùng trong trạng thái license tổng hợp được lấy từ bộ theo dõi kênh sự kiện.
