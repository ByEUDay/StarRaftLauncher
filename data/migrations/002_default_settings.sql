-- 默认设置（值均为 JSON 编码）
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('mirror',              '"bmclapi"'),
    ('downloadConcurrency', '16'),
    ('javaPath',            '"auto"'),
    ('memoryMb',            'null'),
    ('windowWidth',         '854'),
    ('windowHeight',        '480'),
    ('msClientId',          '""');
