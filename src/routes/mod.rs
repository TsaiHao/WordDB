use rusqlite::Connection;
use std::env;
use std::sync::{Arc, Mutex};
use warp;
use warp::Filter;

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
        let db = database_query
            .lock()
            .expect("get database failed when querying");
        let res = db::query_word(&db, word);
        match res {
            Some(def) => warp::reply::json(&def),
            None => warp::reply::json(&"word not found"),
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
        .then(move |json: db::WordEntry| {
            let word = json.word;
            let dict_key = env::var("DICT_KEY").expect("DICT_KEY not found in environment");

            let dict_url = format!("{}{}?key={}", DICT_URL, word, dict_key);
            let database_post = database_post.clone();
            async move {
                let res = reqwest::get(&dict_url).await.expect("request failed");
                let res = res.text().await.expect("json deserialization failed");
                println!("{}", res);

                let db = database_post
                    .lock()
                    .expect("get database failed when putting");
                let add_result = db::add_word(&db, word.clone(), res.clone());
                match add_result {
                    Ok(_) => println!("word added"),
                    Err(e) => println!("add word failed, error = {}", e),
                }

                warp::reply::json(&res)
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
            let res = db::remove_word(&db, word);
            match res {
                Ok(_) => warp::reply::json(&"word removed"),
                Err(e) => warp::reply::json(&format!("remove word failed, error = {}", e)),
            }
        })
}
