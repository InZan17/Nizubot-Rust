use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Write},
    ops::Deref,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use poise::{
    futures_util::lock::Mutex,
    serenity_prelude::{Context, FutureExt, RwLock},
};
use serde::Serialize;

use crate::{give_up_serialize::GiveUpSerialize, Data};

struct DirectoryInfo {
    directories: HashMap<String, DirectoryInfo>,
    data: Option<Box<DataHolderType<dyn GiveUpSerialize + Send + Sync>>>,
}

type DataHolderType<T> = Arc<DataHolder<T>>;

pub struct DataHolder<T: GiveUpSerialize + Send + Sync + 'static + ?Sized> {
    saved: Mutex<bool>,
    path: Vec<String>,
    save_queue: Arc<RwLock<Vec<DataHolderType<dyn GiveUpSerialize + Send + Sync>>>>,
    self_arc: Option<DataHolderType<dyn GiveUpSerialize + Send + Sync>>,
    data_type_id: TypeId,
    data: RwLock<T>,
}

impl<T: GiveUpSerialize + Send + Sync + 'static + ?Sized> DataHolder<T> {
    pub async fn get_data(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        return self.data.read().await;
    }

    pub async fn get_data_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        return self.data.write().await;
    }

    pub async fn request_file_write(&self) {
        *self.saved.lock().await = false;
        self.save_queue
            .write()
            .await
            .push(self.self_arc.clone().unwrap());
    }

    pub async fn delete(&mut self) {}

    pub async fn is_saved(&self) -> bool {
        (*self.saved.lock().await).clone()
    }
}

pub struct StorageManager {
    storage_path: String,
    save_queue: Arc<RwLock<Vec<DataHolderType<dyn GiveUpSerialize + Send + Sync>>>>,
    directories: RwLock<HashMap<String, DirectoryInfo>>,
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
            directories: RwLock::new(HashMap::new()),
            save_queue: Arc::new(RwLock::new(vec![])),
            storage_path,
        }
    }

    pub fn get_full_path(&self, key: String) -> String {
        return format!("{}/{}.json", self.storage_path, key);
    }

    pub fn get_full_directory(&self, key: String) -> String {
        return format!("{}/{}", self.storage_path, key);
    }

    async fn clear_save_queue(&self) {
        let mut save_queue_write = self.save_queue.write().await;
        let save_queue_clone = save_queue_write.clone();
        save_queue_write.clear();
        drop(save_queue_write);

        if save_queue_clone.len() == 0 {
            return;
        }

        println!("Performing save...");

        for data in save_queue_clone {
            if data.is_saved().await {
                println!("{} is already saved", data.path.join("/"));
                continue;
            }

            println!("Saving {}", data.path.join("/"));

            let mut poping_path = data.path.clone();

            let joined_file_path = self.get_full_path(poping_path.join("/"));
            poping_path.pop();
            let joined_file_directory = self.get_full_directory(poping_path.join("/"));

            let file_path = Path::new(&joined_file_path);
            let file_directory = Path::new(&joined_file_directory);

            if !file_directory.exists() {
                fs::create_dir_all(file_directory).unwrap();
            }

            let mut file = File::create(file_path).unwrap();
            let data = data.get_data().await;
            let json_data = data.serialize_json();
            file.write(json_data.as_bytes()).unwrap();

            //local file = fs.open(joined_file_path, "w")
            //fs.write(file, json.stringify(data:read()))
            //fs.close(file)
            //data.saved = true
            //table.insert(savedKeys, key)
        }
    }

    async fn create_data<T: GiveUpSerialize + Send + Sync + for<'de> serde::Deserialize<'de>>(
        &self,
        path: Vec<&str>,
        data: T,
    ) -> DataHolderType<T> {
        let path_string: Vec<String> = path.iter().map(|&s| s.to_owned()).collect();
        let mut data_info = Arc::new(DataHolder {
            saved: Mutex::new(false),
            path: path_string,
            save_queue: self.save_queue.clone(),
            self_arc: None,
            data_type_id: TypeId::of::<DataHolderType<T>>(),
            data: RwLock::new(data),
        });

        // This unsafe block is safe because we know that the Arc has just been created and hasnt been distributed to anywhere else.
        unsafe { Arc::get_mut_unchecked(&mut data_info).self_arc = Some(data_info.clone()) };

        self.register_data(data_info.clone()).await;

        data_info
    }

    async fn register_data<T: GiveUpSerialize + Send + Sync + for<'de> serde::Deserialize<'de>>(
        &self,
        data_holder: DataHolderType<T>,
    ) {
        let mut current_directory: Option<&mut DirectoryInfo> = None;

        let mut self_directories = self.directories.write().await;

        //loop through path and populate the path.
        for key in data_holder.path.clone().into_iter() {
            //if current_directory is some then we check if the key exists.
            //If the key exists then we set current_directory to that key directory.
            //If not then we make a new directory info for that key and make that directory current_directory
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

            //if current_directory is none then that means this is the first iteration.
            //If the key exists then we set current_directory to that key directory.
            //If not then we make a new directory info for that key and make that directory current_directory
            } else {
                current_directory = self_directories.get_mut(&key);
                if current_directory.is_some() {
                    continue;
                }
                let directory = DirectoryInfo {
                    data: None,
                    directories: HashMap::new(),
                };
                self_directories.insert(key.clone(), directory);
                current_directory = self_directories.get_mut(&key);
            }
        }
        //current_directory should now NEVER be None
        current_directory.unwrap().data = Some(Box::new(data_holder));
    }

    pub async fn get_data_or_default<
        T: GiveUpSerialize + Send + Sync + for<'de> serde::Deserialize<'de>,
    >(
        &self,
        mut path: Vec<&str>,
        default_data: T,
    ) -> DataHolderType<T> {
        if path.len() == 0 {
            panic!("Give me a valid path please.");
        }

        let data = self.get_data::<T>(path.clone()).await;

        if data.is_some() {
            return data.unwrap();
        }

        let data_holder = self.create_data(path, default_data).await;

        data_holder
    }

    pub async fn get_data<T: GiveUpSerialize + Send + Sync + for<'de> serde::Deserialize<'de>>(
        &self,
        mut path: Vec<&str>,
    ) -> Option<DataHolderType<T>> {
        if path.len() == 0 {
            return None;
        }

        //First check if it's in the hashmap
        {
            let self_directories = self.directories.read().await;
            let mut current_hashmap = &*self_directories;
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
                if let Some(data) = &directory_info.data {
                    let data_holder_unknown = data.as_ref();
                    if data.data_type_id == TypeId::of::<DataHolderType<T>>() {
                        // This unsafe block is safe, I think. We check if the typeId is the same so surely nothing wrong will happen.
                        let data_holder_cast = unsafe {
                            (data_holder_unknown as &dyn Any)
                                .downcast_ref_unchecked::<DataHolderType<T>>()
                        };
                        return Some(data_holder_cast.clone());
                    }
                    panic!("Key exists but with a different type!")
                }
            }
        }

        //If not in hashmap, we read from files

        let joined_file_path = path.join("/");

        let full_file_path = self.get_full_path(joined_file_path);

        let file_path = Path::new(&full_file_path);

        if file_path.exists() {
            //if path exists then we read from it
            let file = File::open(file_path).unwrap();
            let reader = BufReader::new(file);
            let parsed_data = serde_json::from_reader::<_, T>(reader).unwrap();

            return Some(self.create_data(path, parsed_data).await);
        }

        None
    }
}

pub fn storage_manager_loop(arc_ctx: Arc<Context>, storage_manager: Arc<StorageManager>) {
    tokio::spawn(async move {
        loop {
            storage_manager.clear_save_queue().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}
