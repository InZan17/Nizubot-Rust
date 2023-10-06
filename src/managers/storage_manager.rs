use core::{panic, num::dec2flt::parse};
use std::{any::Any, collections::HashMap, sync::Arc, path::Path, ops::Deref};

use serde::{Deserialize, Serialize};
use tokio::{sync::{RwLock, mpsc::error}, fs};

pub struct DirectoryInfo {
    pub data: Option<Arc<RwLock<DataInfo<dyn Any + Send + Sync>>>>,
    pub directories: HashMap<String, DirectoryInfo>
}

pub struct StorageManager {
    storage_path: String,
    data: Arc<RwLock<HashMap<String, DirectoryInfo>>>,
}

impl StorageManager {
    pub async fn new(storage_path: impl Into<String>) -> Self {
        let storage_path = storage_path.into();
        let path = Path::new(&storage_path);
        if path.to_str().is_none() {
            panic!("Path is not valid UTF-8")
        }
        if !path.exists() {
            fs::create_dir_all(path).await.unwrap();
        }
        StorageManager {
            data: Arc::new(RwLock::new(HashMap::new())),
            storage_path: storage_path.into(),
        }
    }

    pub fn get_full_path(&self, key: String) -> String {
        return format!("{}/{}.json", self.storage_path, key)
    }

    fn create_data<T>(&self, path: Vec<&str>, parsed_data: T, registered: bool) -> DataInfo<T> {
        let data_info = DataInfo::<T>{
            saved: true,
            registered,
            data: Box::new(parsed_data),
            path,
        };
            read = function(self)
                return self.data
            end,
        
            write = function(self, newData)
                self.data = newData
                self.saved = false
                self.storageManager.pendingSaving[self.key] = self
                if self.registered == false then
                    self.registered = true
                    self.storageManager:getTable(self.key).data = self
                end
            end,
        
            delete = function(self)
                self.storageManager:deleteData(self.key)
            end,
        
            isSaved = function(self)
                return self.saved
            end
        }
        return data
    }

    pub async fn get_data<T: for<'a> Deserialize<'a> + Serialize + 'static>(&self, path: Vec<&str>) -> Option<Arc<RwLock<DataInfo<T>>>> {

        if path.len() == 0 {
            return None
        }

        //First check if it's in the hashmap
        {
            let read = self.data.read().await;

            let mut current_hashmap = read.deref();
            let mut current_directory = None;

            //loop through path. If path is loaded current_directory shall be Some(). Else None.
            for key in path {
                current_directory = current_hashmap.get(key);
                if let Some(directory_info) = current_directory {
                    current_hashmap = &directory_info.directories;
                    continue;
                }
                break;
            }
            if let Some(directory_info) = current_directory {
                let any: &dyn Any = &directory_info.data;
                let data_info = any.downcast_ref::<Arc<RwLock<DataInfo<T>>>>();
                if data_info.is_none() {
                    panic!("Tried to get a type with a key but the key had a different type.");
                }
                return data_info.cloned()
            }
            //let v = serde_json::from_str::<T>("ae");
        }

        //If not in hashmap, we read from files
        let file_path = Path::new(&self.get_full_path(path.join("/")));

        if !file_path.exists() {
            return None
        }

        let data_string = fs::read_to_string(file_path).await.unwrap();
        let parsed_data = serde_json::from_str::<T>(&data_string).unwrap();
        self.create_data(path, parsed_data, true);
        keyTable.data = dataTable
        return dataTable


        None
    }

    pub fn get_data_or_default<T: for<'a> Deserialize<'a> + Serialize + 'static>(default_data: T) -> Option<DataInfo<T>> {
        None
    }

    pub fn set_data<T: for<'a> Deserialize<'a> + Serialize + 'static>(data: T) -> Option<DataInfo<T>> {
        None
    }
}

pub struct DataInfo<T: ?Sized> {
    pub saved: bool,
    registered: bool,
    data: Box<T>,
    path: Vec<&str>,
}
