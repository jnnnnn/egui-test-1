use std::collections::{
    btree_map::Entry::{Occupied, Vacant},
    BTreeMap,
};

use crate::db::BookRef;

// Whittle down the list of books by choosing, for each combination of title and
// author, the one with the most recent year.

#[derive(Default)]
pub struct UIFilter {
    seen: BTreeMap<Key, BookIndex>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
struct Key {
    title: String,
    authors: String,
}

struct BookIndex {
    book: BookRef,
    current_index: usize,
    count: usize,
}

fn clean_title(title: &str) -> String {
    title
        .to_lowercase()
        .replace("the ", "")
        .replace("a ", "")
        .replace('\'', "")
        .replace('-', "")
        .replace(' ', "")
        // many titles have junk in braces at the end
        .split(|c: char| c.is_ascii_punctuation())
        .next()
        .unwrap()
        .trim()
        .to_string()
}

pub fn filter_update_booklist(f: &mut UIFilter, books: &mut Vec<BookRef>, newbook: &BookRef) {
    let seen = &mut f.seen;
    let key = Key {
        // strip title of everything after the first non-alphanumeric character
        title: clean_title(&newbook.title),
        // strip authors of everything after the first space
        authors: newbook.authors.split(' ').next().unwrap().to_string(),
    };

    let (new_index, new_count) = match seen.entry(key.clone()) {
        Occupied(bookindex) => {
            let bookindex = bookindex.into_mut();
            bookindex.count += 1;
            if compare(&bookindex.book, newbook) {
                // the new book is better than the old one, replace it
                (bookindex.current_index, bookindex.count + 1)
            } else {
                // there's a better one already in the list, ignore this one
                // duplicates is a RwLock so writing is a little tricky
                match books[bookindex.current_index].duplicates.write() {
                    Ok(mut dups) => *dups = bookindex.count,
                    Err(_) => { /* ignore -- can only happen if poisoned, and panics crash the whole program */
                    }
                }
                return;
            }
        }
        Vacant(_entry) => (books.len(), 1),
    };

    match newbook.duplicates.write() {
        Ok(mut dups) => *dups = new_count,
        Err(_) => { /* ignore as above */ }
    }

    if new_index == books.len() {
        books.push(newbook.clone());
    } else {
        books[new_index] = newbook.clone();
    }

    seen.insert(
        key,
        BookIndex {
            book: newbook.clone(),
            current_index: new_index,
            count: new_count,
        },
    );
}

// return true to replace old with new
fn compare(old: &BookRef, new: &BookRef) -> bool {
    assert_eq!(clean_title(&old.title), clean_title(&new.title));

    // always choose epub if available
    if old.format.eq("epub") != new.format.eq("epub") {
        return new.format.eq("epub");
    }

    // if the new book is more than ten times larger, skip it
    if new.sizeinbytes > old.sizeinbytes * 10 {
        return false;
    }

    // if either publisher starts with "Acrobat", choose the other
    if old.publisher.starts_with("Acrobat") != new.publisher.starts_with("Acrobat") {
        return old.publisher.starts_with("Acrobat");
    }

    // if only one has a publisher, keep that one
    if old.publisher.trim().is_empty() != new.publisher.trim().is_empty() {
        return old.publisher.trim().is_empty();
    }

    let oldyear = old.year.parse::<u32>().unwrap_or(0);
    let newyear = new.year.parse::<u32>().unwrap_or(0);
    if oldyear == newyear {
        // if the years are the same, prefer the one with the most authors
        old.authors.split(',').count() < new.authors.split(',').count()
    } else {
        // otherwise prefer the newer one
        oldyear < newyear
    }
}

// tests
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::db::Book;

    use super::*;

    // keep the one with a publisher, even if the year is newer
    #[test]
    fn test_compare_publisher_year() {
        let old = Arc::new(Book {
            publisher: "".to_string(),
            year: "2002".to_string(),
            ..Default::default()
        });
        let new = Arc::new(Book {
            publisher: "publisher".to_string(),
            year: "2001".to_string(),
            ..Default::default()
        });
        assert!(compare(&old, &new));
        assert!(!compare(&new, &old));
    }
}
