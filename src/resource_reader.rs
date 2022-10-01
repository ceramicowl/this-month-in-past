use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::Metadata;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use chrono::{Local, NaiveDateTime, TimeZone};
use log::error;
use now::DateTimeNow;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};

use crate::{exif_reader, resource_processor};
use crate::geo_location::GeoLocation;
use crate::image_processor::ImageOrientation;

/// A resource reader that reads available resources from the filesystem
#[derive(Clone)]
pub struct ResourceReader {
    /// Holds all specified local paths
    pub local_resource_paths: Vec<String>,
    /// Holds all specified local paths
    pub samba_resource_paths: Vec<String>,
}

impl ResourceReader {
    /// Reads the specified resource from the filesystem
    /// Returns the resource file data
    pub fn read_resource_data(&self, resource: &RemoteResource) -> Vec<u8> {
        match resource.resource_type {
            RemoteResourceType::Local => {
                fs::read(resource.path.clone()).unwrap()
            }
            RemoteResourceType::Samba => {
                // TODO: implement me
                vec![]
            }
        }
    }

    /// Returns all available resources from the filesystem
    pub fn list_all_resources(&self) -> Vec<RemoteResource> {
        let local_resources: Vec<RemoteResource> = self.local_resource_paths
            .par_iter()
            .inspect(|x| println!(" ## local ## {x}")) // TODO: remove me
            .flat_map(|path| read_all_local_files_recursive(&PathBuf::from(path)))
            .map(|resource| exif_reader::fill_exif_data(&resource))
            .collect();

        let samba_resources: Vec<RemoteResource> = self.samba_resource_paths
            .par_iter()
            .inspect(|x| println!(" ## remote ## {x}"))// TODO: remove me
            .flat_map(|path| read_all_samba_files_recursive(&PathBuf::from(path)))
            .map(|resource| exif_reader::fill_exif_data(&resource))
            .collect();

        [local_resources, samba_resources].concat()
    }
}

/// Instantiates a new resource reader for the given paths
pub fn new(resource_folder_paths: &str) -> ResourceReader {
    let mut local_resource_paths = vec![];
    let mut samba_resource_paths = vec![];

    for resource_folder in resource_folder_paths.split(',').map(|s| s.to_string()) {
        if resource_folder.starts_with("smb://") {
            samba_resource_paths.push(resource_folder);
        } else {
            local_resource_paths.push(resource_folder);
        }
    }

    ResourceReader {
        local_resource_paths,
        samba_resource_paths,
    }
}

/// A remote resource that is available on the filesystem
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteResource {
    pub id: String,
    pub path: String,
    pub content_type: String,
    pub name: String,
    pub content_length: u64,
    pub last_modified: NaiveDateTime,
    pub taken: Option<NaiveDateTime>,
    pub location: Option<GeoLocation>,
    pub orientation: Option<ImageOrientation>,
    pub resource_type: RemoteResourceType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RemoteResourceType {
    Local,
    Samba,
}

impl Display for RemoteResourceType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RemoteResourceType::Local => write!(f, "Local"),
            RemoteResourceType::Samba => write!(f, "Samba"),
        }
    }
}

/// Reads all files of a folder and returns all found resources
/// The folder is recursively searched
pub fn read_all_local_files_recursive(folder_path: &PathBuf) -> Vec<RemoteResource> {
    let maybe_folder_path = fs::File::open(folder_path);

    if maybe_folder_path.is_err() {
        error!("Could not open folder: {}", folder_path.display());
        return vec![];
    }

    let metadata = maybe_folder_path
        .unwrap()
        .metadata()
        .expect("Failed to read metadata");

    if metadata.is_file() {
        return vec![];
    }

    let paths = fs::read_dir(folder_path).unwrap_or_else(|_| {
        panic!(
            "Failed to read directory: {}",
            &folder_path.to_str().unwrap()
        )
    });

    paths
        .flatten()
        .flat_map(|dir_entry| {
            let metadata = dir_entry.metadata().expect("Failed to read metadata");

            if metadata.is_file() {
                read_local_file(&dir_entry.path())
            } else {
                read_all_local_files_recursive(&dir_entry.path())
            }
        })
        .collect()
}

/// Reads all files of a samba folder and returns all found resources
/// The folder is recursively searched
pub fn read_all_samba_files_recursive(folder_path: &PathBuf) -> Vec<RemoteResource> {
    // TODO: implement me
    vec![]
}

/// Reads a single file and returns the found resource
/// Checks if the file is a supported resource currently all image types
fn read_local_file(file_path: &PathBuf) -> Vec<RemoteResource> {
    let file = std::fs::File::open(file_path).unwrap();
    let metadata = file.metadata().expect("Failed to read metadata");
    let file_name = file_path.as_path().file_name().unwrap().to_str().unwrap();

    let is_file = metadata.is_file();
    let mime_type: &str = mime_guess::from_path(file_name).first_raw().unwrap_or("");

    // Cancel if no image file
    if !is_file || !mime_type.starts_with("image/") {
        return vec![];
    }

    vec![RemoteResource {
        id: resource_processor::md5(file_name),
        path: file_path.to_str().unwrap().to_string(),
        content_type: mime_type.to_string(),
        name: file_name.to_string(),
        content_length: metadata.len(),
        last_modified: read_last_modified_date(metadata),
        taken: None,
        location: None,
        orientation: None,
        resource_type: RemoteResourceType::Local,
    }]
}

fn read_last_modified_date(metadata: Metadata) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(
        metadata
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::new(0, 0))
            .as_secs() as i64,
        0,
    )
}

impl RemoteResource {
    /// Checks if the resource was taken in the past but in this calendar week
    pub fn is_this_week(&self) -> bool {
        if self.taken.is_none() {
            return false;
        }

        let current_week_of_year = Local::now().week_of_year();
        let resource_week_of_year = Local
            .from_local_datetime(&self.taken.unwrap())
            .unwrap()
            .week_of_year();

        current_week_of_year == resource_week_of_year
    }
}

/// Renders the resource as a string
impl Display for RemoteResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {:?} {:?} {}",
            self.id,
            self.path,
            self.content_type,
            self.name,
            self.content_length,
            self.last_modified,
            self.taken,
            self.location,
            self.resource_type,
        )
    }
}
