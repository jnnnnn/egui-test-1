PRAGMA synchronous = OFF;
PRAGMA journal_mode = MEMORY;

DROP TABLE IF EXISTS `non_fiction_mini`;

CREATE TABLE non_fiction_mini AS 
select fh.ipfs_cid, f.Title, f.Author, f.Series, f.Language, f.Year, f.Publisher, f.Extension, f.Filesize, f.Locator, f.Topic
from updated f
join hashes fh on fh.md5 = lower(f.md5)
where fh.ipfs_cid is not null and trim(fh.ipfs_cid) <> ''
;
CREATE INDEX "idx_non_fiction_mini_Search" ON "non_fiction_mini" (`Author`, `Title`, `Series`, `Language`, `Extension`)
;

drop table description ;
drop table description_edited ;
-- drop table topics ;
drop table hashes ;
drop table updated ;
drop table updated_edited;

VACUUM;

-- 800MB
