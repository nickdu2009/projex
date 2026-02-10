-- 为 persons 表添加 email 和 role 字段

ALTER TABLE persons ADD COLUMN email TEXT NOT NULL DEFAULT '';
ALTER TABLE persons ADD COLUMN role TEXT NOT NULL DEFAULT '';

-- 创建 email 索引以支持快速查找
CREATE INDEX idx_persons_email ON persons(email) WHERE email != '';
