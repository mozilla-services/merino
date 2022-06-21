//! The connection pool for Remote Settings.
//! Due to the lack of sync primitives (i.e. Condvar) in `tokio::sync`, the implementation
//! is based on the counterparts of `std::sync` with certain handling such that blocking
//! operations are performed in a managed manner.

use remote_settings_client::Client;
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// The connection pool for Remote Settings.
pub struct ConnectionPool {
    /// Semaphore to coordinate the connection pool.
    condvar: Condvar,
    /// The underlying connections of the pool.
    connections: Mutex<VecDeque<Client>>,
}

/// The connection to Remote Settings
pub struct Connection<'a> {
    /// The wrapped connection to Remote Settings.
    pub client: Option<Client>,
    /// A reference to the pool, used internally for pool management.
    pool: &'a ConnectionPool,
}

impl ConnectionPool {
    /// Create a new connection pool with the given clients.
    pub fn new(clients: impl IntoIterator<Item = Client>) -> Self {
        Self {
            condvar: Condvar::new(),
            connections: Mutex::new(VecDeque::from_iter(clients)),
        }
    }

    /// Try acquiring a connection from the pool, return `None` if it's not
    /// available without blocking.
    pub fn try_acquire(&self) -> Option<Connection> {
        let res = self.condvar.wait_timeout_while(
            self.connections.lock().unwrap(),
            Duration::from_millis(1),
            |connections| connections.is_empty(),
        );

        match res {
            Ok(mut res) if !res.1.timed_out() => Some(Connection {
                client: res.0.pop_front(),
                pool: self,
            }),
            _ => None,
        }
    }

    /// Acquire a connection from the pool, it will spin until a connection is
    /// acquired from the pool.
    pub async fn acquire(&self) -> Connection<'_> {
        loop {
            match self.try_acquire() {
                Some(connection) => break connection,
                None => {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    continue;
                }
            }
        }
    }

    /// Insert the connection to the pool, this is used to return a connection
    /// back to the pool.
    pub fn insert(&self, client: Client) {
        let mut guard = self.connections.lock().unwrap();
        guard.push_back(client);
        self.condvar.notify_one();
    }
}

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.pool.insert(client);
        }
    }
}

impl std::ops::Deref for Connection<'_> {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}

impl std::ops::DerefMut for Connection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.client.as_mut().unwrap()
    }
}
