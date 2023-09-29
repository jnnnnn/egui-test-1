-- 3 GB

-- 1.6m English, 0.9m everything else

-- from this point on, we become incompatible with the standard layout
-- this requires the compresseddb flag in the config so that the query doesn't try to join to fiction_hashes

/*
CREATE TABLE `updated` ( `ID` int(15) unsigned NOT NULL AUTO_INCREMENT, `Title` varchar(2000) DEFAULT '', `VolumeInfo` varchar(100) DEFAULT '', `Series` varchar(300) DEFAULT '', `Periodical` varchar(200) DEFAULT '', `Author` varchar(1000) DEFAULT '', `Year` varchar(14) DEFAULT '', `Edition` varchar(60) DEFAULT '', `Publisher` varchar(400) DEFAULT '', `City` varchar(100) DEFAULT '', `Pages` varchar(100) DEFAULT '', `PagesInFile` int(10) unsigned NOT NULL DEFAULT 0, `Language` varchar(150) DEFAULT '', `Topic` varchar(500) DEFAULT '', `Library` varchar(50) DEFAULT '', `Issue` varchar(100) DEFAULT '', `Identifier` varchar(300) DEFAULT '', `ISSN` varchar(9) DEFAULT '', `ASIN` varchar(200) DEFAULT '', `UDC` varchar(200) DEFAULT '', `LBC` varchar(200) DEFAULT '', `DDC` varchar(45) DEFAULT '', `LCC` varchar(45) DEFAULT '', `Doi` varchar(45) DEFAULT '', `Googlebookid` varchar(45) DEFAULT '', `OpenLibraryID` varchar(200) DEFAULT '', `Commentary` varchar(10000) DEFAULT '', `DPI` int(6) unsigned DEFAULT 0, `Color` varchar(1) DEFAULT '', `Cleaned` varchar(1) DEFAULT '', `Orientation` varchar(1) DEFAULT '', `Paginated` varchar(1) DEFAULT '', `Scanned` varchar(1) DEFAULT '', `Bookmarked` varchar(1) DEFAULT '', `Searchable` varchar(1) DEFAULT '', `Filesize` bigint(20) unsigned NOT NULL DEFAULT 0, `Extension` varchar(50) DEFAULT '', `MD5` char(32) DEFAULT '', `Generic` char(32) DEFAULT '', `Visible` char(3) DEFAULT '', `Locator` varchar(733) DEFAULT '', `Local` int(10) unsigned DEFAULT 0, `TimeAdded` timestamp NOT NULL DEFAULT '2000-01-01 05:00:00', `TimeLastModified` timestamp NOT NULL DEFAULT current_timestamp() ON UPDATE current_timestamp(), `Coverurl` varchar(200) DEFAULT '', `Tags` varchar(500) DEFAULT '', `IdentifierWODash` varchar(300) DEFAULT '',

CREATE TABLE `topics` (
  `id` integer  NOT NULL PRIMARY KEY AUTOINCREMENT ,  `topic_descr` varchar(500) NOT NULL DEFAULT '' ,  `lang` varchar(2) NOT NULL DEFAULT '' ,  `kolxoz_code` varchar(10) NOT NULL DEFAULT '' ,  `topic_id` integer  DEFAULT NULL ,  `topic_id_hl` integer  DEFAULT NULL );
*/

CREATE TABLE non_fiction_mini AS 
select fh.ipfs_cid, f.Title, f.Author, f.Series, f.Language, f.Year, f.Publisher, f.Extension, f.Filesize, f.Locator, f.Topic
from updated f
join hashes fh on fh.md5 = lower(f.md5)
where fh.ipfs_cid is not null and trim(fh.ipfs_cid) <> ""

CREATE INDEX "idx_non_fiction_mini_Search" ON "non_fiction_mini" (`Author`, `Title`, `Series`, `Language`, `Extension`)

drop table description
drop table description_edited
drop table topics
drop table hashes

VACUUM 

-- 0.5 GB