-- 3 GB

drop table fiction_description 

-- 1.6m English, 0.9m everything else
delete from fiction
where "Language" <> 'English'

-- from this point on, we become incompatible with the standard layout
-- this requires the compresseddb flag in the config so that the query doesn't try to join to fiction_hashes

alter table fiction add column ipfs_cid char(62);

update fiction set ipfs_cid = (select ipfs_cid from fiction_hashes where fiction_hashes.md5 = lower(fiction.md5));

delete from fiction where ipfs_cid IS NULL 

drop table fiction_hashes

-- following https://www.sqlite.org/lang_altertable.html
-- SELECT type, sql FROM sqlite_schema WHERE tbl_name='fiction'

CREATE TABLE `new_fiction` (
  `ID` integer  NOT NULL PRIMARY KEY AUTOINCREMENT
,  `Title` varchar(2000) NOT NULL DEFAULT ''
,  `Author` varchar(300) NOT NULL DEFAULT ''
,  `Series` varchar(300) NOT NULL DEFAULT ''
,  `Language` varchar(45) NOT NULL DEFAULT ''
,  `Year` varchar(10) NOT NULL DEFAULT ''
,  `Publisher` varchar(100) NOT NULL DEFAULT ''
,  `Pages` varchar(10) NOT NULL DEFAULT ''
,  `Coverurl` varchar(200) NOT NULL DEFAULT ''
,  `Extension` varchar(10) NOT NULL
,  `Filesize` integer  NOT NULL
,  `Locator` varchar(512) NOT NULL DEFAULT ''
, ipfs_cid char(62) NOT NULL DEFAULT ''
)

INSERT INTO new_fiction SELECT `ID`, Title, Author, Series, `Language`, `Year`, Publisher, Pages, Coverurl, Extension, Filesize, `Locator`, ipfs_cid FROM fiction

drop table fiction

ALTER TABLE new_fiction RENAME TO fiction

CREATE INDEX "idx_fiction_Search" ON "fiction" (`Author`, `Title`, `Series`, `Language`, `Extension`)

VACUUM 

-- 0.5 GB