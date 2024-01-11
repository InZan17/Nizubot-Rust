use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
    sync::Arc, time::Duration,
};

use poise::{
    futures_util::lock::Mutex,
    serenity_prelude::{Context, RwLock},
};
use tokio::{io::{AsyncWriteExt, AsyncReadExt}};

use crate::{give_up_serialize::GiveUpSerialize, Error};

pub struct DataDirectories {}
impl DataDirectories {
    pub fn cotd_guilds() -> Vec<&'static str> {
        vec!["cotd_guilds"]
    }
}

pub fn storage_manager_loop(_arc_ctx: Arc<Context>, storage_manager: Arc<StorageManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}


pub enum DataType {
    String(String),
    Bytes(Vec<u8>)
}
pub struct DataHolder {
    path: String,
    data: DataType,
}

impl DataType {
    fn bytes(&self) -> &[u8] {
        match self {
            DataType::String(str) => str.as_ref(),
            DataType::Bytes(bytes) => bytes.as_ref(),
        }
    }

    pub fn get_string(self) -> String {
        match self {
            DataType::String(string) => string,
            DataType::Bytes(bytes) => panic!("IT WASNT A STRING!!!"),
        }
    }
}

pub struct StorageManager {
    pub storage_path: String,
    datas: RwLock<HashMap<String, (Duration, Arc<RwLock<DataType>>)>>,
}

impl StorageManager {

    pub async fn new(storage_path: impl Into<String>) -> Self {
        let storage_path = storage_path.into();
        let path = Path::new(&storage_path);
        if path.to_str().is_none() {
            panic!("Path is not valid UTF-8")
        }
        if !path.exists() {
            fs::create_dir(path).unwrap();
        }
        StorageManager {
            storage_path,
            datas: RwLock::new(HashMap::new()),
        }
    }

    pub async fn save_mem(self: &Arc<Self>, key: &str, data: Arc<RwLock<DataType>>, duration: Duration) {
        let mut write = self.datas.write().await;
        write.insert(key.into(), (duration, data));
    }

    pub async fn load_mem(self: &Arc<Self>, key: &str) -> Option<Arc<RwLock<DataType>>> {
        let read = self.datas.read().await;
        let (_, data) = read.get(key)?;
        Some(data.clone())
    }

    pub async fn load_mem_or(self: &Arc<Self>, key: &str, data: DataType, duration: Duration) -> Arc<RwLock<DataType>> {
        let Some(mem_data) = self.load_mem(key).await else {
            let data = Arc::new(RwLock::new(data));
            return data
        };

        mem_data
    }

    pub async fn delete_mem(self: &Arc<Self>, key: &str) {
        let mut write = self.datas.write().await;
        write.remove(key);
    }

    pub async fn save_disk(self: &Arc<Self>, path: &str, data: DataType) -> Result<String, Error> {

        let path = self.get_full_directory(path);

        if let Some(path) = Path::new(&path).parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(data.bytes()).await?;

        Ok(path)
    }

    pub async fn load_disk(self: &Arc<Self>, path: &str, to_string: bool) -> Result<Option<DataType>, Error> {

        let path = self.get_full_directory(path);

        if !Path::new(&path).exists() {
            return Ok(None)
        }

        let mut file = tokio::fs::File::open(path).await?;

        let mut buffer = Vec::new();

        let _len = file.read(&mut buffer).await?;

        if to_string {
            let convert = std::str::from_utf8(&buffer)?;
            return Ok(Some(DataType::String(convert.to_owned())))
        } else {
            return Ok(Some(DataType::Bytes(buffer)))
        }
    }

    pub async fn load_disk_or(self: &Arc<Self>, path: &str, to_string: bool, data: DataType) -> Result<DataType, Error> {
        let Some(data) = self.load_disk(path, to_string).await? else {
            return Ok(data)
        };
        Ok(data)
    }

    pub async fn delete_disk(self: &Arc<Self>, path: &str) -> Result<String, Error> {
        let path = self.get_full_directory(path);
        let path_path = Path::new(&path);

        if !path_path.exists() {
            return Ok(path)
        }

        tokio::fs::remove_file(&path).await?;
        return Ok(path)
    }

    pub fn get_full_directory(&self, path: &str) -> String {
        return format!("{}/{}", self.storage_path, path);
    }

    pub fn get_full_file_path(&self, path: &str, extension: &str) -> String {
        return format!("{}/{}.{}", self.storage_path, path, extension);
    }
}