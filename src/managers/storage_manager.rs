use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use poise::{
    futures_util::lock::Mutex,
    serenity_prelude::{Context, RwLock},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{give_up_serialize::GiveUpSerialize, utils::get_seconds, Error};

pub struct DataDirectories {}
impl DataDirectories {
    pub fn cotd_guilds() -> Vec<&'static str> {
        vec!["cotd_guilds"]
    }
}

//TODO simple docs for the functions

pub fn storage_manager_loop(_arc_ctx: Arc<Context>, storage_manager: Arc<StorageManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let mut removals = vec![];

            let copied = storage_manager.datas.read().await.clone();

            for (key, data) in copied.iter() {
                if data.0 < get_seconds() {
                    removals.push(key);
                }
            }

            let mut write = storage_manager.datas.write().await;

            for key in removals {
                write.remove(key);
            }
        }
    });
}

#[derive(Clone, Debug)]
pub enum DataType {
    String(String),
    Bytes(Vec<u8>),
}
pub struct DataHolder {
    pub path: String,
    pub data: Arc<RwLock<DataType>>,
}

impl DataType {
    fn as_bytes(&self) -> &[u8] {
        match self {
            DataType::String(str) => str.as_ref(),
            DataType::Bytes(bytes) => bytes.as_ref(),
        }
    }

    pub fn string(&self) -> Option<&String> {
        match self {
            DataType::String(string) => Some(string),
            DataType::Bytes(bytes) => None,
        }
    }

    pub fn bytes(&self) -> Option<&Vec<u8>> {
        match self {
            DataType::String(string) => None,
            DataType::Bytes(bytes) => Some(bytes),
        }
    }

    pub fn string_mut(&mut self) -> Option<&mut String> {
        match self {
            DataType::String(string) => Some(string),
            DataType::Bytes(bytes) => None,
        }
    }

    pub fn bytes_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            DataType::String(string) => None,
            DataType::Bytes(bytes) => Some(bytes),
        }
    }
}

pub struct StorageManager {
    pub storage_path: String,
    datas: RwLock<HashMap<String, (u64, Arc<RwLock<DataType>>)>>,
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

    pub async fn save_mem(
        self: &Arc<Self>,
        key: &str,
        data: Arc<RwLock<DataType>>,
        duration: Duration,
    ) {
        let mut write = self.datas.write().await;
        write.insert(key.into(), (duration_to_timestamp(&duration), data));
    }

    pub async fn load_mem(self: &Arc<Self>, key: &str) -> Option<Arc<RwLock<DataType>>> {
        let read = self.datas.read().await;
        let (_, data) = read.get(key)?;
        Some(data.clone())
    }

    pub async fn load_mem_or(
        self: &Arc<Self>,
        key: &str,
        data: DataType,
        duration: Duration,
    ) -> Arc<RwLock<DataType>> {
        let Some(mem_data) = self.load_mem(key).await else {
            let data = Arc::new(RwLock::new(data));
            self.save_mem(key, data.clone(), duration).await;
            return data;
        };

        mem_data
    }

    pub async fn delete_mem(self: &Arc<Self>, key: &str) {
        let mut write = self.datas.write().await;
        write.remove(key);
    }

    pub async fn save_disk(self: &Arc<Self>, path: &str, data: &DataType) -> Result<String, Error> {
        let path = self.get_full_directory(path);

        if let Some(path) = Path::new(&path).parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(data.as_bytes()).await?;

        Ok(path)
    }

    pub async fn load_disk(
        self: &Arc<Self>,
        path: &str,
        to_string: bool,
    ) -> Result<Option<DataType>, Error> {
        let path = self.get_full_directory(path);

        if !Path::new(&path).exists() {
            return Ok(None);
        }

        let mut file = tokio::fs::File::open(path).await?;

        if to_string {
            let mut string = String::new();
            file.read_to_string(&mut string).await?;
            return Ok(Some(DataType::String(string)));
        } else {
            let mut buffer = Vec::new();
            let _len = file.read(&mut buffer).await?;
            return Ok(Some(DataType::Bytes(buffer)));
        }
    }

    pub async fn load_disk_or(
        self: &Arc<Self>,
        path: &str,
        to_string: bool,
        data: DataType,
    ) -> Result<DataType, Error> {
        let Some(data) = self.load_disk(path, to_string).await? else {
            self.save_disk(path, &data);
            return Ok(data);
        };
        Ok(data)
    }

    pub async fn delete_disk(self: &Arc<Self>, path: &str) -> Result<String, Error> {
        let path = self.get_full_directory(path);
        let path_path = Path::new(&path);

        if !path_path.exists() {
            return Ok(path);
        }

        tokio::fs::remove_file(&path).await?;
        return Ok(path);
    }

    pub async fn save(
        self: &Arc<Self>,
        path: &str,
        data_holder: &DataHolder,
        duration: Duration,
    ) -> Result<String, Error> {
        let read = data_holder.data.read().await;
        let result = self.save_disk(path, &read).await;

        let duration = if result.is_err() {
            // we do this to make sure we arent loosing data.
            Duration::MAX
        } else {
            duration
        };

        self.save_mem(path, data_holder.data.clone(), duration)
            .await;

        result
    }

    pub async fn load(
        self: &Arc<Self>,
        path: &str,
        to_string: bool,
        duration: Duration,
    ) -> Result<Option<DataHolder>, Error> {
        if let Some(data) = self.load_mem(path).await {
            println!("using mem");
            return Ok(Some(DataHolder {
                path: path.to_string(),
                data,
            }));
        }

        let option = self.load_disk(path, to_string).await?;

        if let Some(data) = option {
            println!("using disk");
            let data = Arc::new(RwLock::new(data));
            self.save_mem(path, data.clone(), duration).await;
            return Ok(Some(DataHolder {
                path: path.to_string(),
                data,
            }));
        }

        println!("using none");

        Ok(None)
    }

    pub async fn load_or(
        self: &Arc<Self>,
        path: &str,
        to_string: bool,
        data: DataType,
        duration: Duration,
    ) -> Result<DataHolder, Error> {
        let res = self.load(path, to_string, duration).await?;

        if let Some(data_holder) = res {
            return Ok(data_holder);
        };

        let arc_data = Arc::new(RwLock::new(data));

        let data = DataHolder {
            path: path.to_string(),
            data: arc_data,
        };

        self.save(path, &data, duration).await;

        Ok(data)
    }

    pub async fn delete(self: &Arc<Self>, path: &str) -> Result<String, Error> {
        self.delete_mem(path).await;
        self.delete_disk(path).await
    }

    pub fn get_full_directory(&self, path: &str) -> String {
        return format!("{}/{}", self.storage_path, path);
    }
}

pub fn duration_to_timestamp(duration: &Duration) -> u64 {
    duration.as_secs().saturating_add(get_seconds())
}
