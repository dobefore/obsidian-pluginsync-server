CREATE TABLE IF NOT EXISTS meta (
    id INTEGER PRIMARY KEY ,
    fname TEXT NOT NULL,
    -- index to content
    indexs INTEGER NOT NULL,
    states TEXT NOT NULL,
    ctime INTEGER NOT NULL,
    mtime INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS content (
    id INTEGER PRIMARY KEY,
    content TEXT NOT NULL
);