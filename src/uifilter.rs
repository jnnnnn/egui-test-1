use std::collections::BTreeMap;

use crate::db::Book;

// Whittle down the list of books by choosing, for each combination of title and
// author, the one with the most recent year.

#[derive(Default)]
pub struct UIFilter {
    seen: BTreeMap<Key, BookRef>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
struct Key {
    title: String,
    authors: String,
}

struct BookRef {
    book: Book,
    current_index: usize,
}

pub fn filter_update_booklist(f: &mut UIFilter, books: &mut Vec<Book>, newbook: &Book) {
    let seen = &mut f.seen;
    let key = Key {
        // strip title of everything after the first non-alphanumeric character
        title: newbook
            .title
            .split(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap()
            .to_string(),
        // strip authors of everything after the first space
        authors: newbook.authors.split(' ').next().unwrap().to_string(),
    };

    let new_index: usize = match seen.get(&key) {
        Some(bookref) => {
            if compare(&bookref.book, newbook) {
                // the new book is better than the old one, replace it
                bookref.current_index
            } else {
                // there's a better one already in the list, ignore this one
                return;
            }
        }
        None => books.len(),
    };

    if new_index == books.len() {
        books.push(newbook.clone());
    } else {
        books[new_index] = newbook.clone();
    }

    seen.insert(
        key,
        BookRef {
            book: newbook.clone(),
            current_index: new_index,
        },
    );
}

// return true to replace old with new
fn compare(old: &Book, new: &Book) -> bool {
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
    use once_cell::sync::Lazy;

    use super::*;

    // make a base book to clone for tests
    static BASE: Lazy<Book> = Lazy::<Book, _>::new(|| Book {
        title: "title".to_string(),
        authors: "author".to_string(),
        year: "2000".to_string(),
        publisher: "publisher".to_string(),
        sizeinbytes: 100,
        collection: crate::db::Collection::Fiction,
        series: "Discworld 5".to_string(),
        language: "en".to_string(),
        format: "epub".to_string(),
        ipfs_cid: "Qm123".to_string(),
    });

    // keep the one with a publisher, even if the year is newer
    #[test]
    fn test_compare_publisher_year() {
        let old = Book {
            publisher: "".to_string(),
            year: "2002".to_string(),
            ..BASE.clone()
        };
        let new = Book {
            publisher: "publisher".to_string(),
            year: "2001".to_string(),
            ..BASE.clone()
        };
        assert!(compare(&old, &new));
        assert!(!compare(&new, &old));
    }

}
