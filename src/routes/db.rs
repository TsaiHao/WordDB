use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Rusqlite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Word not found")]
    WordNotFound,

    #[error("Word duplicate")]
    WordDuplicate,
}

/// Represents the definition of a word.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WordDefinition {
    fl: String,
    shortdef: Vec<String>,
}

/// Represents an entry for a word in the database.
#[derive(Deserialize, Serialize, Debug)]
pub struct WordEntry {
    /// The word.
    pub word: String,
    /// The definition of the word.
    pub definition: Option<String>,
    /// The date when the word was added.
    pub date: Option<String>,
}

/// Represents the response for a word query.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordResponse {
    result: String,
    word: Option<String>,
    definition: Option<Vec<WordDefinition>>,
    message: Option<String>,
    suggestions: Option<Vec<String>>,
}

/// Retrieves a list of all words from the database.
///
/// # Arguments
///
/// * `db` - The database connection.
///
/// # Returns
///
/// A vector containing all the words in the database.
pub fn list_all_words(db: &Connection) -> Vec<String> {
    let list = db.prepare("SELECT word FROM words");
    match list {
        Ok(mut stat) => {
            let iter = stat
                .query_map([], |row| {
                    let word: String = row.get(0).expect("get 0 failed");
                    Ok(word)
                })
                .expect("query map failed");

            let mut words = Vec::new();
            for word_result in iter {
                if let Ok(word) = word_result {
                    words.push(word);
                }
            }

            words
        }
        Err(e) => {
            println!("list all words failed, error = {}", e);
            Vec::new()
        }
    }
}

/// Queries a word from the database.
///
/// # Arguments
///
/// * `db` - The database connection.
/// * `word` - The word to query.
///
/// # Returns
///
/// An `Option` containing the `WordEntry` if the word is found, or `None` if the word is not found.
pub fn query_word(db: &Connection, word: String) -> Option<WordEntry> {
    let mut stmt = db
        .prepare("SELECT word,definition,date FROM words where word = ?1")
        .expect("prepare db failed");
    let row = stmt.query_row(
        params![word],
        |row| {
            let word: String = row.get(0).expect("get 0 failed");
            let definition: String = row.get(1).expect("get 1 failed");
            let date: String = row.get(2).expect("get 2 failed");
            Ok(WordEntry {
                word: word,
                definition: Some(definition),
                date: Some(date),
            })
        },
    );

    match row {
        Ok(entry) => {
            Some(entry)
        }
        Err(e) => {
            println!("query failed, error = {}", e);
            None
        }
    }
}

/// Adds a word to the database.
///
/// # Arguments
///
/// * `db` - The database connection.
/// * `word` - The word to add.
/// * `definition` - The definition of the word.
pub fn add_word(db: &Connection, word: String, definition: String) -> Result<(), DatabaseError> {
    let entry = query_word(db, word.clone());
    if entry.is_some() {
        println!("word already exists = {}", word);
        return Err(DatabaseError::WordDuplicate);
    }

    let mut stmt = db
        .prepare("INSERT INTO words (word, definition, date) VALUES (?1, ?2, ?3)")
        .expect("prepare db failed");
    let date = chrono::Utc::now().to_string();
    let add_result = stmt.execute(params![word, definition, date]);
    match add_result {
        Ok(_) => {
            println!("add word success = {}", word);
            Ok(())
        },
        Err(e) => {
            println!("add word failed, error = {}", e);
            Err(DatabaseError::SqliteError(e))
        }
    }
}

/// Removes a word from the database.
///
/// # Arguments
///
/// * `db` - The database connection.
/// * `word` - The word to remove.
///
/// # Returns
///
/// An `Result` indicating success or failure.
pub fn remove_word(db: &Connection, word: String) -> Result<(), DatabaseError> {
    let entry = query_word(db, word.clone());
    if entry.is_none() {
        println!("word not found = {}", word);
        return Err(DatabaseError::WordNotFound);
    }

    let mut stmt = db
        .prepare("DELETE FROM words WHERE word = ?1")
        .expect("prepare db failed");
    let remove_result = stmt.execute(params![word]);
    match remove_result {
        Ok(_) => {
            println!("remove word success = {}", word);
            Ok(())
        },
        Err(e) => {
            println!("remove word failed, error = {}", e);
            Err(DatabaseError::SqliteError(e))
        }
    }
}
