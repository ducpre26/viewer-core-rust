drop table if exists session cascade;
CREATE TABLE session
(
    id                     varchar(64) primary key,
    store_url              text,
    store_auth             text,
    studies                text not null,
    user_id                varchar(32),
    anonymized             boolean not null default false,
    shared_session         boolean not null default true,
    share_key              varchar(16),
    expired_time           timestamp with time zone not null,
    permissions            text[]
);
create index idx_session_expired_time on session (expired_time);

drop table if exists config cascade;
CREATE TABLE config
(
    id                     varchar(64) primary key,
    value                  text not null,
    description            text,
    datatype               varchar(32)
);

drop table if exists client cascade;
CREATE TABLE client
(
    id            varchar(64) primary key,
    password      varchar(64) not null,
    ip_allowed    text
);

drop table if exists permission cascade;
CREATE TABLE permission
(
    id          varchar(64) primary key,
    name        varchar(64) not null unique,
    description text
);

-- Thêm permission cơ bản
INSERT INTO permission (id, name, description) VALUES
    ('MODIFY_OVERLAY_CONFIG', 'Modify Overlay Config', 'Cho phép sửa cấu hình overlay DICOM'),
    ('ADMIN', 'Admin', 'Có quyền thực hiện tất cả các thao tác')
ON CONFLICT DO NOTHING;

drop table if exists client_permission cascade;
CREATE TABLE client_permission
(
    client_id     varchar(64) not null references client(id) on delete cascade,
    permission_id varchar(64) not null references permission(id) on delete cascade,
    primary key (client_id, permission_id)
);


drop table if exists study_key_image cascade;
CREATE TABLE study_key_image
(
    id                varchar primary key not null,
    series_iuid       varchar(64)              not null,
    num_of_image      integer                  not null default 0,
    store_url         text                     not null,
    store_auth        text,
    succeed           boolean                  not null default false,
    num_of_retry      integer                  not null default 0,
    description       text,

    created_time      timestamp with time zone not null,
    creator_id        integer                  not null default 1,
    last_updated_time timestamp with time zone,
    last_updated_id   integer
);

drop table if exists models cascade;
CREATE TABLE models
(
    id                serial primary key,
    model_name        varchar(255) not null unique,
    url               text not null,
    authentication    text,
    config_json       jsonb,

    created_time      timestamp with time zone not null default now(),
    creator_id        integer not null default 1,
    last_updated_time timestamp with time zone,
    last_updated_id   integer
);

-- Drop table nếu tồn tại
DROP TABLE IF EXISTS printer CASCADE;

-- Tạo bảng printer
CREATE TABLE printer
(
    id                serial PRIMARY KEY,
    printer_name      varchar(255) NOT NULL,
    host              varchar(255) NOT NULL,
    port              integer NOT NULL,
    aetitle           varchar(255) NOT NULL,
    called_aetitle    varchar(255) NOT NULL,
    description       text,
    enabled           boolean NOT NULL,

    created_time      timestamp with time zone NOT NULL DEFAULT now(),
    creator_id        integer NOT NULL DEFAULT 1,
    last_updated_time timestamp with time zone,
    last_updated_id   integer
);


