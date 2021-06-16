//! Utilities to run tests against a Redis server by using multiple databases to
//! run tests in parallel.

use anyhow::{ensure, Result};
use deadpool::unmanaged::{Object, Pool};
use lazy_static::lazy_static;
use redis::ConnectionInfo;

use tokio::sync::OnceCell;

lazy_static! {
    static ref AVAILABLE_DATABASES: OnceCell<Pool<i64>> = OnceCell::const_new();
}

/// Get a connection info that uses an unused database on the Redis server.
///
/// To define "unused", this assumes that no other programs are using Redis
/// databases greater than 0. It is safe to run multiple tests in parallel within
/// one test runner, but not to run multiple test runners.
///
/// The return value contains a connection info to use as normal, and a guard
/// object. When the guard is dropped, the database used by that connection info
/// is marked as available again. It is up to the caller to ensure that the
/// connection info does not outlive the guard.
pub async fn get_temp_db(
    connection_info: &ConnectionInfo,
) -> Result<(ConnectionInfo, Object<i64>)> {
    ensure!(
        connection_info.db == 0,
        "template connection must use database 0"
    );

    let pool = AVAILABLE_DATABASES
        .get_or_try_init(|| async {
            let mut client = redis::Client::open(connection_info.clone())?;
            let response: Vec<String> = redis::cmd("CONFIG")
                .arg("GET")
                .arg("databases")
                .query(&mut client)?;

            assert_eq!(response.len(), 2);
            assert_eq!(response[0], "databases");
            let max_dbs = response[1].parse()?;

            let db_ids: Vec<i64> = (1..max_dbs).collect();
            let pool = Pool::from(db_ids);
            assert_eq!(pool.status().available as i64, max_dbs - 1);
            Result::<_, anyhow::Error>::Ok(pool)
        })
        .await?;

    let guard = pool.get().await?;
    let mut connection_info = connection_info.clone();
    connection_info.db = *guard.as_ref();

    let mut client =
        redis::Client::open(connection_info.clone()).expect("Couldn't open Redis connection");
    let _: () = redis::cmd("FLUSHDB")
        .query(&mut client)
        .expect("Couldn't clear test Redis DB");

    Ok((connection_info, guard))
}
