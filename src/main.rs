use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server, StatusCode};
use log::LevelFilter;
use quaint::{pooled::Quaint, prelude::*};

use futures::future::join_all;
use simple_logger::SimpleLogger;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("msg {0}")]
    QueryError(String),
}

type PrismaResult<T> = Result<T, Error>;

async fn stats_logger() {
    let mut builder = Quaint::builder("postgresql://postgres:prisma@localhost:5434").unwrap();
    builder.connection_limit(1);
    let pool = builder.build();

    let sleep_duration = Duration::from_secs(1);

    loop {
        sleep(sleep_duration).await;
        let conn = pool.check_out().await.unwrap();
        let count = Select::from_table("pg_stat_activity")
            .columns(vec!["usename", "application_name", "state", "state_change"])
            .so_that("datname".equals("postgres"));
        let res = conn.select(count).await.unwrap();
        println!("Stats=> connections: {:?}", res.len() - 1);
    }
}

async fn get_posts(pool: Arc<Quaint>, id: String) -> PrismaResult<i64> {
    let conn = match pool.check_out().await {
        Err(_) => return Err(Error::QueryError("pool query timeout".to_string())),
        Ok(conn) => conn,
    };

    let post_count = Select::from_table("post")
        .so_that("userId".equals(id.as_str()))
        .value(count(asterisk()));
    let post = conn.select(post_count).await;

    if post.is_err() {
        println!("ERROR {:?}", post);
        return Err(Error::QueryError("post query error".to_string()));
    }

    let count = post
        .unwrap()
        .first()
        .unwrap()
        .get("count")
        .unwrap()
        .as_i64()
        .unwrap();

    Ok(count)
}

async fn get_ids(pool: Arc<Quaint>) -> PrismaResult<ResultSet> {
    let conn = match pool.check_out().await {
        Err(_) => return Err(Error::QueryError("user pool query timeout".to_string())),
        Ok(conn) => conn,
    };

    let user_id_query = Select::from_table("user")
        .column("id")
        .column("name")
        // .limit(1)
        // .so_that("1".equals("1"))
        .offset(0);

    let res = conn.select(user_id_query).await;

    if let Ok(result_set) = res {
        Ok(result_set)
    } else {
        println!("ERR {:?}", res);
        Err(Error::QueryError("user pool query".to_string()))
    }
}

async fn route(pool: Arc<Quaint>, _req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let ids = match get_ids(pool.clone()).await {
        Err(err) => {
            println!("IDS {:?}", err);
            return Ok(Response::builder()
                .status(501)
                .body(Body::from("{}"))
                .unwrap());
        }
        Ok(result_set) => result_set,
    };

    let mut futures = Vec::with_capacity(50);
    for row in ids {
        let id = row.get("id").unwrap().as_str().unwrap().to_string();
        let count_fut = get_posts(pool.clone(), id);
        futures.push(count_fut);
    }

    let results = join_all(futures).await;

    let mut counts: Vec<i64> = Vec::new();
    for result in results {
        if result.is_err() {
            return Ok(Response::builder()
                .status(500)
                .body(Body::from("{}"))
                .unwrap());
        }
        let count = result.unwrap();
        counts.push(count);
    }

    let json = serde_json::to_vec(&counts).unwrap();
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .unwrap();
    Ok(response)
}

pub async fn listen() -> PrismaResult<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        // .with_module_level("mobc", LevelFilter::Debug)
        // .with_module_level("quaint", LevelFilter::Debug)
        .init()
        .unwrap();

    tokio::task::spawn(stats_logger());
    let mut builder = Quaint::builder("postgresql://postgres:prisma@localhost:5434").unwrap();

    builder.health_check_interval(Duration::from_secs(1));
    builder.connection_limit(10);
    builder.test_on_check_out(true);
    builder.max_idle_lifetime(Duration::from_secs(1));
    builder.max_idle(2);
    builder.max_lifetime(Duration::from_secs(1));

    let pool = Arc::new(builder.build());

    // let p = pool.clone();
    // tokio::task::spawn(async move {
    //     loop {
    //         sleep(Duration::from_secs(2)).await;
    //         let s = p.inner.state().await;
    //         println!("pool-stats {:?}", s);
    //     }
    // });

    let query_engine = make_service_fn(move |_| {
        let state = pool.clone();
        async move { Ok::<_, hyper::Error>(service_fn(move |req| route(state.clone(), req))) }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));

    let server = Server::bind(&addr).tcp_nodelay(true).serve(query_engine);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    console_subscriber::init();
    listen().await
}

/*
node

Server Software:
Server Hostname:        127.0.0.1
Server Port:            4000

Document Path:          /
Document Length:        236 bytes

Concurrency Level:      200
Time taken for tests:   120.004 seconds
Complete requests:      15449
Failed requests:        0
Total transferred:      6859356 bytes
HTML transferred:       3645964 bytes
Requests per second:    128.74 [#/sec] (mean)
Time per request:       1553.547 [ms] (mean)
Time per request:       7.768 [ms] (mean, across all concurrent requests)
Transfer rate:          55.82 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    1   1.5      0      11
Processing:   193 1542 137.6   1537    3267
Waiting:      189 1533 136.0   1529    3158
Total:        194 1543 137.6   1538    3268

Percentage of the requests served within a certain time (ms)
  50%   1538
  66%   1555
  75%   1564
  80%   1572
  90%   1587
  95%   1602
  98%   1654
  99%   2142
 100%   3268 (longest request)

*/

/* rust
Finished 15701 requests


Server Software:
Server Hostname:        127.0.0.1
Server Port:            4000

Document Path:          /
Document Length:        226 bytes

Concurrency Level:      500
Time taken for tests:   120.004 seconds
Complete requests:      15701
Failed requests:        0
Total transferred:      5259835 bytes
HTML transferred:       3548426 bytes
Requests per second:    130.84 [#/sec] (mean)
Time per request:       3821.534 [ms] (mean)
Time per request:       7.643 [ms] (mean, across all concurrent requests)
Transfer rate:          42.80 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    5  33.4      0     405
Processing:    38 3750 420.7   3791    4578
Waiting:       38 3749 420.7   3791    4578
Total:         47 3754 411.2   3793    4579

Percentage of the requests served within a certain time (ms)
  50%   3793
  66%   3864
  75%   3907
  80%   3940
  90%   4032
  95%   4110
  98%   4183
  99%   4236
 100%   4579 (longest request)



*/

/*
Server Software:
Server Hostname:        127.0.0.1
Server Port:            4000

Document Path:          /
Document Length:        226 bytes

Concurrency Level:      200
Time taken for tests:   120.028 seconds
Complete requests:      3698
Failed requests:        0
Total transferred:      1238830 bytes
HTML transferred:       835748 bytes
Requests per second:    30.81 [#/sec] (mean)
Time per request:       6491.515 [ms] (mean)
Time per request:       32.458 [ms] (mean, across all concurrent requests)
Transfer rate:          10.08 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    2  13.9      0     103
Processing:    87 6315 859.0   6474    6936
Waiting:       83 6315 859.0   6474    6936
Total:         87 6318 856.2   6475    6936

Percentage of the requests served within a certain time (ms)
  50%   6475
  66%   6545
  75%   6580
  80%   6598
  90%   6642
  95%   6676
  98%   6729
  99%   6751
 100%   6936 (longest request)


*/
