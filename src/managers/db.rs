use std::{time::Duration, future::IntoFuture};

use surrealdb::{Surreal, engine::remote::ws::{Client, Ws}, opt::auth::Database};

use crate::{tokens, Error};

pub trait IsConnected {
    async fn is_connected(&self) -> Result<(), Error>;
}

impl IsConnected for Surreal<Client> {
    async fn is_connected(&self) -> Result<(), Error> {
        let future = IntoFuture::into_future(self.version());
        let res_res = tokio::time::timeout(Duration::from_millis(100), future).await;
        let Ok(res) = res_res else {
            return Err(Error::from("Not connected to database."))
        };

        res?;

        Ok(())
    }
}

pub async fn new_db() -> Surreal<Client> {
    let surreal_login_info = tokens::get_surreal_signin_info();

    let db = Surreal::new::<Ws>(surreal_login_info.address).await.expect("Couldn't connect to SurrealDB. Please make sure you got internet or that the database is up.");

    let a = db.use_ns(&surreal_login_info.namespace).use_db(&surreal_login_info.database).await.expect("Failed to set ns and db.");

    let a = db
        .signin(Database {
            username: &surreal_login_info.username,
            password: &surreal_login_info.password,
            namespace: &surreal_login_info.namespace,
            database: &surreal_login_info.database,
        })
        .await.expect("Failed to sign in.");

    db
}