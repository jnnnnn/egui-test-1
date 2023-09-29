PRAGMA synchronous = OFF;
PRAGMA journal_mode = MEMORY;

CREATE TABLE fiction_mini AS 
select fh.ipfs_cid, f.Title, f.Author, f.Series, f.Language, f.Year, f.Publisher, f.Extension, f.Filesize, f.Locator
from fiction f
join fiction_hashes fh on fh.md5 = lower(f.md5)
where fh.ipfs_cid is not null and trim(fh.ipfs_cid) <> ''
;
CREATE INDEX "idx_fiction_mini_Search" ON "fiction_mini" (`Author`, `Title`, `Series`, `Language`, `Extension`)
;
drop table fiction ;
drop table fiction_description ;
drop table fiction_hashes ;

VACUUM;

-- 0.5 GB