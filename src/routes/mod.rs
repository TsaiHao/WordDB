use rusqlite::Connection;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use warp;
use warp::Filter;
use warp::http::{ Response, StatusCode };

pub mod db;

const DICT_URL: &str = "https://www.dictionaryapi.com/api/v3/references/collegiate/json/";

pub fn list_route(
    database: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let database_list = database.clone();
    warp::path!("api" / "word" / "list").map(move || {
        println!("list route");
        let database_list = database_list.lock().unwrap();
        let res = db::list_all_words(&database_list);
        warp::reply::json(&res)
    })
}

pub fn query_route(
    database: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let database_query = database.clone();
    warp::path!("api" / "word" / String).map(move |word: String| {
        println!("query route {}", word);
        let db = database_query
            .lock()
            .expect("get database failed when querying");
        let res = db::query_word(&db, word.clone());
        match res {
            Some(def) => Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&def).unwrap())
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "application/json")
                .body(format!("{{\"error\": \"{} not found\"}}", word))
                .unwrap(),
        }
    })
}

pub fn insert_route(
    database: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let database_post = database.clone();
    warp::path!("api" / "word")
        .and(warp::post())
        .and(warp::body::json())
        .then(move |word: db::WordEntry| {
            let word = word.word;
            let word = word.to_lowercase();
            let dict_key = env::var("DICT_KEY").expect("DICT_KEY not found in environment");

            let dict_url = format!("{}{}?key={}", DICT_URL, word, dict_key);
            println!("query word {}", dict_url);
            let database_post = database_post.clone();
            async move {
                let res = reqwest::get(&dict_url).await.expect("request failed");
                let status = res.status();
                let res = res.text().await.expect("json deserialization failed");

                if res.starts_with("[\"") || res == "[]" || status != StatusCode::OK {
                    println!("query word failed");
                    let mut body: HashMap<String, String> = HashMap::new();
                    if status != StatusCode::OK {
                        body.insert("error".to_string(), "request failed".to_string());
                    } else {
                        body.insert("error".to_string(), "word not found".to_string());
                        body.insert("suggestion".to_string(), res);
                    }
                    return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "application/json")
                            .body(serde_json::to_string(&body).unwrap())
                            .unwrap();
                }

                println!("query word suceed {}", res);

                let db = database_post
                    .lock()
                    .expect("get database failed when putting");
                let add_result = db::add_word(&db, word.clone(), res.clone());
                match add_result {
                    Ok(_) => println!("word added"),
                    Err(e) => println!("add word failed, error = {}", e),
                }

                let res: db::WordEntry = db::WordEntry {
                    word: word,
                    definition: Some(res),
                    date: None,
                };

                Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&res).unwrap())
                    .unwrap()
            }
        })
}

pub fn delete_route(
    database: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let database_delete = database.clone();
    warp::path!("api" / "word" / String)
        .and(warp::delete())
        .map(move |word: String| {
            let db = database_delete
                .lock()
                .expect("get database failed when deleting");
            let res = db::remove_word(&db, word.clone());
            match res {
                Ok(_) => Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(format!("{{\"word\": \"{}\"}}", word))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(format!("{{\"error\": \"{}\"}}", e))
                    .unwrap(),
            }
        })
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use rusqlite::Connection;

    fn get_db() -> Arc<Mutex<Connection>> {
        let db = Connection::open_in_memory().expect("Failed to open database");
        db.execute(
            "CREATE TABLE IF NOT EXISTS words (word TEXT PRIMARY KEY, definition TEXT, date TEXT)",
            [],
        )
        .expect("Failed to create table");
        Arc::new(Mutex::new(db))
    }

    fn get_test_word_list() -> Vec<String> {
        vec![
            "counsel".to_string(),
            "stunned".to_string(),
            "catastrophic".to_string(),
            "hurdle".to_string(),
            "dismal".to_string(),
            "jittery".to_string(),
        ]
    }

    #[tokio::test]
    async fn test_insert_route() {
        println!("entering test");
        let db = get_db();
        let insert_route = super::insert_route(db.clone());

        let words = get_test_word_list();
        for word in &words {
            let req = warp::test::request()
                .method("POST")
                .path("/api/word")
                .json(&super::db::WordEntry {
                    word: word.clone(),
                    definition: None,
                    date: None,
                })
                .reply(&insert_route)
                .await;

            assert_eq!(req.status(), 200);
        }

        let list_route = super::list_route(db.clone());

        let req = warp::test::request()
            .method("GET")
            .path("/api/word/list")
            .reply(&list_route)
            .await;

        assert_eq!(req.status(), 200);
        let word_list = get_test_word_list();
        let res: Vec<String> = serde_json::from_slice(&req.body()).unwrap();
        println!("recieve list: {:?}", res);
        for word in word_list {
            assert!(res.iter().any(|entry| *entry == word));
        }

        let delete_route = super::delete_route(db.clone());
        for word in &words {
            let req = warp::test::request()
                .method("DELETE")
                .path(&format!("/api/word/{}", word))
                .reply(&delete_route)
                .await;

            assert_eq!(req.status(), 200);
        }

        let req = warp::test::request()
            .method("GET")
            .path("/api/word/list")
            .reply(&list_route)
            .await;

        assert_eq!(req.status(), 200);
        let res: Vec<String> = serde_json::from_slice(&req.body()).unwrap();
        assert_eq!(res.len(), 0);
    }
}