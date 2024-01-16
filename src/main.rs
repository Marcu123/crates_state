use serde_derive::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

struct SharedData {
    crate_versions: Mutex<HashMap<String, u32>>,
    crate_most_versions: Mutex<(String, u32)>,
    crate_most_features: Mutex<(String, u32, Vec<String>)>,
    crate_dependants: Mutex<HashMap<String, HashSet<String>>>,
    crate_most_dependencies: Mutex<(String, u32, Vec<String>)>,
}

impl SharedData {
    fn new() -> Self {
        SharedData {
            crate_versions: Mutex::new(HashMap::new()),
            crate_most_versions: Mutex::new((String::new(), 0)),
            crate_most_features: Mutex::new((String::new(), 0, Vec::new())),
            crate_dependants: Mutex::new(HashMap::new()),
            crate_most_dependencies: Mutex::new((String::new(), 0, Vec::new())),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Crate {
    name: String,
    deps: Vec<Dependency>,
    features: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct Dependency {
    name: String,
}

fn most_dependencies(path: &Path, shared_data: Arc<SharedData>, thread_id: u32) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut max_dependencies = shared_data.crate_most_dependencies.lock().unwrap();

    for line in reader.lines() {
        let line = line?;
        match serde_json::from_str::<Crate>(&line) {
            Ok(crate_info) => {
                let dependency_count = crate_info.deps.len() as u32;

                if dependency_count > max_dependencies.1 {
                    max_dependencies.0 = crate_info.name.clone();
                    max_dependencies.1 = dependency_count;
                    max_dependencies.2 =
                        crate_info.deps.iter().map(|dep| dep.name.clone()).collect();
                }
            }
            Err(err) => {
                eprintln!(
                    "Eroare la deserializarea liniei: {}, path: {}",
                    err,
                    path.display()
                );
                continue;
            }
        }
    }

    println!(
        "Thread-ul {} a terminat de procesat fișierul {}",
        thread_id,
        path.display()
    );
    Ok(())
}

fn most_dependants(path: &Path, shared_data: Arc<SharedData>, thread_id: u32) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        match serde_json::from_str::<Crate>(&line) {
            Ok(crate_info) => {
                for dep in crate_info.deps.iter() {
                    let mut crate_dependants = shared_data.crate_dependants.lock().unwrap();
                    crate_dependants
                        .entry(dep.name.clone())
                        .or_default()
                        .insert(crate_info.name.clone());
                }
            }
            Err(err) => {
                eprintln!(
                    "Eroare la deserializarea liniei: {}, path: {}",
                    err,
                    path.display()
                );
                continue;
            }
        }
    }

    println!(
        "Thread-ul {} a procesat fișierul {}",
        thread_id,
        path.display()
    );
    Ok(())
}

fn most_features(path: &Path, shared_data: Arc<SharedData>, thread_id: u32) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut max_features = shared_data.crate_most_features.lock().unwrap();

    for line in reader.lines() {
        let line = line?;
        match serde_json::from_str::<Crate>(&line) {
            Ok(crate_info) => {
                let feature_count = crate_info.features.len() as u32;

                if feature_count > max_features.1 {
                    max_features.0 = crate_info.name.clone();
                    max_features.1 = feature_count;
                    max_features.2 = crate_info.features.keys().cloned().collect();
                }
            }
            Err(err) => {
                eprintln!(
                    "Eroare la deserializarea liniei: {}, path: {}",
                    err,
                    path.display()
                );
                continue;
            }
        }
        println!(
            "Thread-ul {} a terminat de procesat fișierul {}",
            thread_id,
            path.display()
        );
    }
    Ok(())
}

fn most_versions(path: &Path, shared_data: Arc<SharedData>, thread_id: u32) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut max_versions = shared_data.crate_most_versions.lock().unwrap();
    let mut versions = shared_data.crate_versions.lock().unwrap();

    for line in reader.lines() {
        let line = line?;
        match serde_json::from_str::<Crate>(&line) {
            Ok(crate_info) => {
                let crate_versions = versions.entry(crate_info.name.clone()).or_insert(0);
                *crate_versions += 1;

                if *crate_versions > max_versions.1 {
                    max_versions.0 = crate_info.name;
                    max_versions.1 = *crate_versions;
                }
            }
            Err(err) => {
                eprintln!(
                    "Eroare la deserializarea liniei: {}, path: {}",
                    err,
                    path.display()
                );
                continue;
            }
        }
    }

    println!(
        "Thread-ul {} a terminat de procesat fișierul {}",
        thread_id,
        path.display()
    );
    Ok(())
}

fn file_analysis(path: &Path, thread_id: u32, shared_data: Arc<SharedData>) -> io::Result<()> {
    if path.is_file() {
        if thread_id == 1 {
            most_dependencies(path, shared_data.clone(), thread_id)?;
        }

        if thread_id == 2 {
            most_dependants(path, shared_data.clone(), thread_id)?;
        }

        if thread_id == 3 {
            most_features(path, shared_data.clone(), thread_id)?;
        }

        if thread_id == 4 {
            most_versions(path, shared_data.clone(), thread_id)?;
        }
    }

    Ok(())
}

fn recursive_folder_analysis(
    path: &Path,
    thread_id: u32,
    shared_data: Arc<SharedData>,
) -> io::Result<()> {
    if path.is_dir()
        && (path.file_name().unwrap() != ".git"
            && path.file_name().unwrap() != "tmp"
            && path.file_name().unwrap() != ".github")
    {
        for entry in path.read_dir()? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                recursive_folder_analysis(&path, thread_id, Arc::clone(&shared_data))?;
            } else {
                file_analysis(&path, thread_id, Arc::clone(&shared_data))?;
            }
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let shared_data = Arc::new(SharedData::new());
    let path_start = Path::new("crates-demo");

    let mut handles = vec![];

    for i in 1..=4 {
        let shared_data = shared_data.clone();
        let path = path_start.to_path_buf();
        let handle = thread::spawn(move || {
            recursive_folder_analysis(&path, i, shared_data).unwrap_or_else(|err| {
                eprintln!("Eroare în thread-ul {}: {}", i, err);
            });
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut file = File::create("output.txt")?;

    let most_dependencies = shared_data.crate_most_dependencies.lock().unwrap();
    writeln!(
        file,
        "Crate-ul cu cele mai multe dependente: {} ({})",
        most_dependencies.0, most_dependencies.1
    )
    .unwrap();

    writeln!(file, "Lista dependentelor: {:?}", most_dependencies.2).unwrap();
    writeln!(file).unwrap();

    let crate_dependants = shared_data.crate_dependants.lock().unwrap();
    let (most_dependants_crate, dependants) = crate_dependants
        .iter()
        .max_by_key(|(_, dependants)| dependants.len())
        .map(|(name, dependants)| (name.clone(), dependants.clone()))
        .unwrap_or_else(|| (String::new(), HashSet::new()));

    writeln!(
        file,
        "Crate-ul cu cei mai multi dependenti: {} ({})",
        most_dependants_crate,
        dependants.len()
    )
    .unwrap();
    writeln!(file, "Lista dependentilor: {:?}", dependants).unwrap();
    writeln!(file).unwrap();

    let most_features = shared_data.crate_most_features.lock().unwrap();
    writeln!(
        file,
        "Crate-ul cu cele mai multe features: {} ({})",
        most_features.0, most_features.1
    )
    .unwrap();

    writeln!(file, "Lista features-urilor: {:?}", most_features.2).unwrap();
    writeln!(file).unwrap();

    let most_versions = shared_data.crate_most_versions.lock().unwrap();
    writeln!(
        file,
        "Crate-ul cu cele mai multe versiuni: {} ({})",
        most_versions.0, most_versions.1
    )
    .unwrap();
    writeln!(file).unwrap();

    Ok(())
}
