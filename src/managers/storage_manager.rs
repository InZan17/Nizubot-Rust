use core::panic;
use std::{
    any::{type_name, type_name_of_val, Any, TypeId},
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use poise::serenity_prelude::RwLock;

pub type ArcDataInfo = Arc<RwLock<DataInfo<Box<dyn Any + Send + Sync>>>>;

pub struct DirectoryInfo {
    pub data: Option<Box<Arc<RwLock<DataInfo<Box<dyn Any + Sync + Send>>>>>>,
    pub directories: HashMap<String, DirectoryInfo>,
}

pub struct StorageManager {
    storage_path: String,
    data: Arc<RwLock<HashMap<String, DirectoryInfo>>>,
    save_queue: Arc<RwLock<Vec<Box<dyn Any + Sync + Send>>>>,
}

impl StorageManager {
    pub async fn new(storage_path: impl Into<String>) -> Self {
        let storage_path = storage_path.into();
        let path = Path::new(&storage_path);
        if path.to_str().is_none() {
            panic!("Path is not valid UTF-8")
        }
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }
        StorageManager {
            data: Arc::new(RwLock::new(HashMap::new())),
            save_queue: Arc::new(RwLock::new(vec![])),
            storage_path: storage_path.into(),
        }
    }

    pub fn get_full_path(&self, key: String) -> String {
        return format!("{}/{}.json", self.storage_path, key);
    }

    async fn create_data<T: Any + Send + Sync>(
        &self,
        path: Vec<&str>,
        parsed_data: Box<T>,
    ) -> Arc<RwLock<DataInfo<Box<T>>>> {
        let path_string = path.iter().map(|&s| s.to_owned()).collect();
        let data_info = DataInfo {
            saved: false,
            data: Box::new(parsed_data),
            path: path_string,
            self_arc: None,
            unsaved_list: self.save_queue.clone(),
        };

        let data_info_arc = Arc::new(RwLock::new(data_info));

        let push_data: Box<Arc<RwLock<DataInfo<Box<T>>>>> = Box::new(data_info_arc.clone());

        self.register_data(data_info_arc.clone()).await;

        self.save_queue.write().await.push(push_data);

        return data_info_arc;
    }

    async fn register_data<T: Any + Send + Sync>(&self, arc_data: Arc<RwLock<DataInfo<Box<T>>>>) {
        let path = &arc_data.read().await.path;
        let mut write = self.data.write().await;

        let mut current_directory: Option<&mut DirectoryInfo> = None;

        //loop through path and populate the path.
        for key in path.clone().into_iter() {
            if let Some(prev_directory) = current_directory {
                if prev_directory.directories.get_mut(&key).is_some() {
                    current_directory = prev_directory.directories.get_mut(&key);
                    continue;
                }
                let directory = DirectoryInfo {
                    data: None,
                    directories: HashMap::new(),
                };
                prev_directory.directories.insert(key.clone(), directory);
                current_directory = prev_directory.directories.get_mut(&key);
            } else {
                current_directory = write.get_mut(&key);
                if current_directory.is_some() {
                    continue;
                }
                let directory = DirectoryInfo {
                    data: None,
                    directories: HashMap::new(),
                };
                write.insert(key.clone(), directory);
                current_directory = write.get_mut(&key);
            }
        }
        //let push_data: Option<Box<Arc<RwLock<DataInfo<Box<T>>>>>> = Some(Box::new(arc_data.clone()));
        current_directory.unwrap().data = Some(Box::new(arc_data.clone()));
    }

    pub async fn get_data<T>(&self, path: Vec<&str>) -> Option<Box<Arc<RwLock<DataInfo<Box<T>>>>>>
    where
        T: Serialize + Any + Send + Sync + for<'de> serde::Deserialize<'de>,
    {
        if path.len() == 0 {
            return None;
        }

        //First check if it's in the hashmap
        {
            let read = self.data.read().await;

            let mut current_hashmap = read.deref();
            let mut current_directory = None;

            //loop through path. If path is loaded current_directory shall be Some(). Else None.
            for key in path.clone().into_iter() {
                current_directory = current_hashmap.get(key);
                if let Some(directory_info) = current_directory {
                    current_hashmap = &directory_info.directories;
                    continue;
                }
                break;
            }
            if let Some(directory_info) = current_directory {
                let directory_data = &directory_info.data;
                let asref = directory_info.data.as_ref().unwrap();
                let any: &dyn Any = asref;
                println!("{}", type_name::<T>());
                //this is really not good omg please someone help me fix rhis I have no idea why the unsafe function works but not the safe one.
                let data_info =
                    unsafe { any.downcast_ref_unchecked::<Box<Arc<RwLock<DataInfo<Box<T>>>>>>() };
                //if data_info.is_none() {
                //    panic!("Tried to get a type with a key but the key had a different type.");
                //}
                return Some(data_info.clone());
            }
            //let v = serde_json::from_str::<T>("ae");
        }

        //If not in hashmap, we read from files

        let joined_path = path.join("/");
        let full_path = self.get_full_path(joined_path);
        println!("full_path: {}", full_path);
        let file_path = Path::new(&full_path);

        if !file_path.exists() {
            return None;
        }

        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);
        let read = serde_json::from_reader::<_, T>(reader).unwrap();
        let parsed_data = Box::new(read);

        let data_info = self.create_data(path, parsed_data).await;
        let return_data: Option<Box<Arc<RwLock<DataInfo<Box<T>>>>>> = Some(Box::new(data_info));
        return return_data;

        None
    }

    pub fn get_data_or_default<T: for<'a> Deserialize<'a> + Serialize>(
        default_data: T,
    ) -> Option<DataInfo<T>> {
        None
    }

    pub fn set_data<T: for<'a> Deserialize<'a> + Serialize>(data: T) -> Option<DataInfo<T>> {
        None
    }
}

pub struct DataInfo<T: ?Sized> {
    saved: bool,
    data: Box<T>,
    self_arc: Option<ArcDataInfo>,
    unsaved_list: Arc<RwLock<Vec<Box<dyn Any + Sync + Send>>>>,
    path: Vec<String>,
}

impl<T> DataInfo<T> {
    pub fn get_data(&self) -> &T {
        return &self.data;
    }

    pub fn get_data_mut(&mut self) -> &mut T {
        return &mut self.data;
    }

    pub fn overwrite(&mut self, data: T) {
        self.data = Box::new(data);
    }

    pub async fn request_file_write(&mut self) {
        self.saved = false;
        self.unsaved_list
            .write()
            .await
            .push(Box::new(self.self_arc.clone().unwrap()));
        /*if self.registered == false then
            self.registered = true
            self.storageManager:getTable(self.key).data = self
        end*/
    }

    pub async fn delete(&mut self) {}

    pub fn is_saved(&self) -> bool {
        self.saved
    }
}
