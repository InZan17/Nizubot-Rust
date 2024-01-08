use std::{future::IntoFuture, time::Duration};

use serde::Deserialize;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Database,
    sql::Thing,
    Surreal,
};

use crate::{tokens, Error};

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

//TODO: Every query, when it returns something you should always use take() or take_errors() to make sure no errors happened

pub trait IsConnected {
    /// Returns an error if it isnt connected and an Ok if it is connected.
    /// The reason it returns a result is because then we can use the ? thing.
    async fn is_connected(&self) -> Result<(), Error>;
}

impl IsConnected for Surreal<Client> {
    async fn is_connected(&self) -> Result<(), Error> {
        let future = IntoFuture::into_future(self.version());
        let res_res = tokio::time::timeout(Duration::from_millis(100), future).await;
        let Ok(res) = res_res else {
            return Err(Error::from("Not connected to database."));
        };

        res?;

        Ok(())
    }
}

pub async fn new_db() -> Surreal<Client> {
    let surreal_login_info = tokens::get_surreal_signin_info();

    let db = Surreal::new::<Ws>(surreal_login_info.address).await.expect("Couldn't connect to SurrealDB. Please make sure you got internet or that the database is up.");

    let a = db
        .use_ns(&surreal_login_info.namespace)
        .use_db(&surreal_login_info.database)
        .await
        .expect("Failed to set ns and db.");

    let a = db
        .signin(Database {
            username: &surreal_login_info.username,
            password: &surreal_login_info.password,
            namespace: &surreal_login_info.namespace,
            database: &surreal_login_info.database,
        })
        .await
        .expect("Failed to sign in.");

    db
}
