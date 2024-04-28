use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use warp;
use warp::Filter;
use tokio;

pub mod routes;

fn main() {
    let database = Connection::open("words.db").expect("Failed to open database");
    database
        .execute(
            "CREATE TABLE IF NOT EXISTS words (word TEXT PRIMARY KEY, definition TEXT, date TEXT)",
            [],
        )
        .expect("Failed to create table");

    let database = Arc::new(Mutex::new(database));

    let list_route = routes::list_route(database.clone());

    let query_route = routes::query_route(database.clone());

    let insert_route = routes::insert_route(database.clone());
    
    let delete_route = routes::delete_route(database.clone());

    let routes = list_route
        .or(query_route)
        .or(insert_route)
        .or(delete_route);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on( async {
        println!("server started at 0.0.0.0:5678");
        warp::serve(routes).run(([0, 0, 0, 0], 5678)).await;
    });
}
