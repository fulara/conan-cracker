use std::path::{Path, PathBuf};

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Output};
use structopt::StructOpt;

use serde::{Deserialize, Serialize};

#[macro_use]
extern crate error_chain;

mod conan_package;
mod err;

use crate::conan_package::*;
use std::fs::File;
use walkdir::{DirEntry, Error};
/*
whats next:
default wrap is missing - deduction!
conan install shouldnt be returning output the conan wrapper should be analyzing the output
conan install should print the output (possibly only in case of error )

next revision:
add ability to install out of other db.
 */

#[derive(StructOpt, Debug)]
#[structopt(name = "cracker")]
enum Opt {
    Install(OptInstall),
}

#[derive(StructOpt, Debug)]
struct OptInstall {
    reference: String,
    #[structopt(long)]
    wrappers: Vec<String>,
    #[structopt(long)]
    prefix: Option<PathBuf>,
    #[structopt(long)]
    bin_dir: Option<PathBuf>,
    #[structopt(long, short)]
    settings: Vec<String>,
    #[structopt(long, short)]
    options: Vec<String>,
    #[structopt(long)]
    generate_enable: bool,
}

struct Paths {
    prefix: PathBuf,
    bin_dir: PathBuf,
}

impl Paths {
    fn storage_dir(&self) -> PathBuf {
        self.prefix.join(".cracker_storage")
    }

    fn db_path(&self) -> PathBuf {
        self.prefix.join(".cracker_index")
    }

    fn generate_install_folder(&self, pkg_name: &str) -> PathBuf {
        //so.. it would be better to actully randomize this - but for now its okayish.
        //but it doesnt handle at all installing mulple version of the packages.
        self.storage_dir().join(pkg_name)
    }
}
fn execute(mut c: Command) -> std::io::Result<std::process::Output> {
    c.output()
}

struct ConanStorageGuard<Executor: Fn(Command) -> std::io::Result<std::process::Output>> {
    executor: Executor,
    original_storage: String,
}

impl<Executor: Fn(Command) -> std::io::Result<std::process::Output>> ConanStorageGuard<Executor> {
    pub fn new(executor: Executor, storage_path: &Path) -> Self {
        let guard = Self {
            original_storage: Self::get_storage_path(&executor),
            executor,
        };

        Self::set_storage_path(
            &guard.executor,
            storage_path
                .as_os_str()
                .to_str()
                .expect("Guard::new Path not str?"),
        );

        guard
    }

    fn get_storage_path(executor: &Executor) -> String {
        let mut c = Command::new("conan");
        c.args(&["config", "get", "storage.path"]);
        let output = executor(c).expect(&format!("Unable to extract result of get storage path"));
        std::str::from_utf8(&output.stdout)
            .expect(&format!("Borked output from get storage path."))
            .to_owned()
    }

    fn set_storage_path(executor: &Executor, path: &str) {
        let mut c = Command::new("conan");
        c.args(&["config", "set", "storage.path", path]);
        executor(c).expect(&format!("Unable to set storage path!"));
    }
}

impl<Executor: Fn(Command) -> std::io::Result<std::process::Output>> Drop
    for ConanStorageGuard<Executor>
{
    fn drop(&mut self) {
        Self::set_storage_path(&self.executor, &self.original_storage);
    }
}

struct Conan<Executor> {
    executor: Executor,
}

impl<Executor: Clone + Fn(Command) -> std::io::Result<std::process::Output>> Conan<Executor> {
    fn new(executor: Executor) -> err::Result<Self> {
        Ok(Self { executor })
    }

    fn install(
        &self,
        conan_pkg: &ConanPackage,
        paths: &Paths,
        install_folder: &str,
        settings: Vec<String>,
        options: Vec<String>,
    ) -> err::Result<Output> {
        let guard = ConanStorageGuard::new(self.executor.clone(), &paths.storage_dir());
        let settings: Vec<&str> = settings
            .iter()
            .flat_map(|s| vec!["-s", s.as_ref()])
            .collect();
        let options: Vec<&str> = options
            .iter()
            .flat_map(|o| vec!["-o", o.as_ref()])
            .collect();
        let mut c = Command::new("conan");
        c.args(&["install", &conan_pkg.full()])
            .args(&["-if", install_folder])
            .args(&["-g", "virtualrunenv", "-g", "virtualenv"])
            .args(&settings)
            .args(&options);
        Ok((self.executor)(c)?)
    }
}

fn init_cache<Fs: filesystem::FileSystem>(
    fs: &Fs,
    paths: &Paths,
) -> err::Result<(CrackerDatabase)> {
    //we need to load cache here.
    let db = if fs.is_file(paths.db_path()) {
        CrackerDatabase::load(fs, paths.db_path())?
    } else {
        fs.create_dir_all(paths.storage_dir())?;
        CrackerDatabase::new()
    };
    fs.create_dir_all(paths.bin_dir.clone())?;

    Ok(db)
}

fn generate_enable_script<Fs: filesystem::FileSystem>(fs: &Fs, paths: &Paths) -> err::Result<()> {
    let path = paths
        .bin_dir
        .parent()
        .ok_or("Unable to extract parent path")?
        .join("cracker_enable");

    fs.write_file(
        path,
        format!(
            r#"
#!/bin/bash
export PATH="{}:$PATH"
"#,
            paths.bin_dir.display()
        )
        .trim()
        .to_owned(),
    );

    Ok(())
}

fn input<R: Read>(mut reader: BufReader<R>, message: &'_ impl std::fmt::Display) -> bool {
    loop {
        println!("{}", message);
        let mut ans = String::new();
        reader
            .read_line(&mut ans)
            .expect("Failed to read from stdin");

        let ans = ans.to_lowercase();

        if &ans == "y" || &ans == "yes" {
            return true;
        } else if &ans == "n" || &ans == "no" {
            return false;
        } else {
            println!("only [y|yes|n|no] is accepted as an answer.")
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Wrapper {
    wrapped_bin: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CrackerDatabaseEntry {
    conan_pkg: ConanPackage,
    conan_settings: Vec<String>,
    conan_options: Vec<String>,
    wrappers: Vec<Wrapper>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CrackerDatabase {
    wrapped: Vec<CrackerDatabaseEntry>,
    storage_owned_by: String,
}

impl CrackerDatabase {
    fn new() -> Self {
        CrackerDatabase {
            wrapped: vec![],
            storage_owned_by: whoami::username(),
        }
    }
    fn load<Fs: filesystem::FileSystem>(fs: &Fs, path: PathBuf) -> err::Result<Self> {
        let content = &fs.read_file(path)?;
        let loaded: Self = serde_json::from_slice(&content)?;

        if loaded.storage_owned_by != whoami::username() {
            Err(err::ErrorKind::CrackerStorageDifferentUsername(
                loaded.storage_owned_by,
                whoami::username(),
            )
            .into())
        } else {
            Ok(loaded)
        }
    }

    fn save(&self, path: PathBuf) -> err::Result<()> {
        let ser = serde_json::to_string(self)?;
        use std::fs;
        Ok(File::create(path)?.write_all(ser.as_bytes())?)
    }

    fn wrapped(&self, wrapper_name: &str) -> Option<(&Wrapper)> {
        self.wrapped
            .iter()
            .find_map(|e| e.wrappers.iter().find(|w| &w.wrapped_bin == wrapper_name))
    }

    fn register_wrap(&mut self, binary: &str, req: &CrackRequest) {
        let e_opt = self.wrapped.iter_mut().find(|entry| {
            req.pkg == entry.conan_pkg
                && req.options == entry.conan_options
                && req.settings == entry.conan_settings
        });
        let e = if let Some(e) = e_opt {
            e
        } else {
            self.wrapped.push(CrackerDatabaseEntry {
                conan_pkg: req.pkg.clone(),
                wrappers: vec![],
                conan_settings: req.options.to_vec(),
                conan_options: req.settings.to_vec(),
            });
            self.wrapped.last_mut().unwrap()
        };

        e.wrappers.push(Wrapper {
            wrapped_bin: binary.to_owned(),
        });
    }
}

struct CrackRequest {
    pkg: ConanPackage,
    bin_name: String,
    settings: Vec<String>,
    options: Vec<String>,
}

fn crack<R: Read, Fs: filesystem::FileSystem>(
    reader: BufReader<R>,
    fs: &Fs,
    request: &CrackRequest,
    paths: &Paths,
    db: &mut CrackerDatabase,
) -> std::io::Result<()> {
    let wrapper_path = paths.bin_dir.join(&request.bin_name);
    if let Some(wrapper) = db.wrapped(&request.bin_name) {
        if !input(
            reader,
            &format!("Wrapper {} already generated overwrite?", request.bin_name,),
        ) {
            return Ok(());
        }

        fs.remove_file(&wrapper_path)?;
    }

    fs.write_file(
        &wrapper_path,
        format!(
            r#"
#!/bin/bash
source {pkg_dir}/activate_run.sh
source {pkg_dir}/activate.sh
{bin_name} "${{@}}"
"#,
            pkg_dir = paths.bin_dir.display(),
            bin_name = request.bin_name
        )
        .trim()
        .to_owned(),
    )?;

    db.register_wrap(&request.bin_name, request);

    Ok(())
}

fn expand_mode_to_all_users(mode: u32) -> u32 {
    let lit = 0o700 & mode;
    mode | (lit >> 3) | (lit >> 6)
}

fn bump_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let meta =
        std::fs::metadata(path).expect(&format!("Unable to get metadata for {}", path.display()));
    let mut permissions = meta.permissions();
    let curr_mode = permissions.mode();
    let expanded_mode = expand_mode_to_all_users(curr_mode);
    if curr_mode != expanded_mode {
        permissions.set_mode(expanded_mode);
        std::fs::set_permissions(path, permissions).expect(&format!(
            "Unable to set permissions for: {}",
            path.display()
        ))
    }
}

fn extract_path<Fs : filesystem::FileSystem>(fs : &Fs, path : PathBuf) -> Option<String> {
    let content = fs.read_file(path).ok()?;
    let content = std::str::from_utf8(&content).ok()?;
    for line in content.lines() {
        if line.starts_with("PATH=") {
            let regex = regex::Regex::new(r#"^PATH="([^"]+)."#)
                .expect("Path deduction regex was invalid.");
            let captures = regex.captures(line).expect("Installed binary didnt have proper PATH?");
            let path = captures.get(1).expect("Installed binary didnt have proper PATH?");

            return Some(path.as_str().to_owned());
        }
    }

    None
}

fn do_install(env_path: Option<PathBuf>, i: OptInstall) -> err::Result<()> {
    let prefix = i
        .prefix
        .or(env_path)
        .expect("provide either prefix or define CRACKER_STORAGE_DIR env.");

    let mut selected_bin_dir = prefix.clone();
    selected_bin_dir.push("bin");
    let bin_dir_env = std::env::var("CRACKER_STORAGE_BIN")
        .ok()
        .map(|p| PathBuf::from(p));
    let bin_dir = i
        .bin_dir
        .or(bin_dir_env)
        .or(Some(selected_bin_dir))
        .unwrap();

    let fs = filesystem::OsFileSystem::new();
    let paths = Paths { prefix, bin_dir };

    let db = init_cache(&fs, &paths)?;

    if i.generate_enable {
        generate_enable_script(&fs, &paths);
    }

    let conan = Conan::new(execute)?;
    let conan_pkg = ConanPackage::new(&i.reference)?;
    let if_path = paths.generate_install_folder(&conan_pkg.name);
    let install_folder = if_path
        .as_os_str()
        .to_str()
        .ok_or("unable to generate if folder")?;
    conan.install(&conan_pkg, &paths, &install_folder, i.settings, i.options)?;

    let env_run_path = if_path.join("environment_run.sh.env");
    let path = extract_path(&fs, env_run_path).expect("environment_run.sh.env did not contain correct PATH? non binary package requested?");

    for entry in walkdir::WalkDir::new(path) {
        match entry {
            Ok(entry) => {
                let p = entry.path();
                use std::os::unix::fs::PermissionsExt;
                if 0o100 & std::fs::metadata(p).expect("unable to extract metadata").permissions().mode() != 0 {
                    // gene
                    asdsadsadsa finish this...
                }
            }
            Err(e) => {
                println!("got error while iterating: {}", e);
            }
        }
    }



    //wrap.
    for entry in walkdir::WalkDir::new(paths.storage_dir()) {
        match entry {
            Ok(entry) => {
                let p = entry.path();
                bump_permissions(p);
            }
            Err(e) => {
                println!("got error while iterating: {}", e);
            }
        }
    }

    db.save(paths.db_path());

    Ok(())
}

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:#?}", opt);

    let env_path = std::env::var("CRACKER_STORAGE_DIR")
        .ok()
        .map(|p| PathBuf::from(p));

    match opt {
        Opt::Install(i) => {
            if let Err(e) = do_install(env_path, i) {
                match e.0 {
                    err::ErrorKind::Io(_) => {
                        panic!("io error: {}", e.0);
                    }
                    err::ErrorKind::SerdeJson(_) => {
                        panic!("Serde Json Error: {}", e.0);
                    }
                    err::ErrorKind::Msg(_) => {}
                    err::ErrorKind::ConanNotInPath => {
                        println!("{}", e);
                    }
                    err::ErrorKind::CrackerStorageDifferentUsername(_, _) => {
                        println!("{}", e);
                    }
                    err::ErrorKind::__Nonexhaustive {} => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod package_tests {
    use crate::conan_package::ConanPackage;
    use crate::{crack, err, expand_mode_to_all_users, generate_enable_script, init_cache, Conan, CrackRequest, CrackerDatabase, CrackerDatabaseEntry, Paths, Wrapper, extract_path};
    use std::collections::BTreeMap;
    use std::io::BufReader;
    use std::path::PathBuf;
    use std::process::Command;

    fn assert_command_generate_output(
        c: Command,
        sender: std::sync::mpsc::Sender<String>,
        stdout: &str,
    ) -> std::io::Result<std::process::Output> {
        let invocation = format!("{:?}", c);
        sender.send(invocation);

        use std::os::unix::process::ExitStatusExt;
        use std::process::{ExitStatus, Output};
        Ok(Output {
            status: ExitStatus::from_raw(0i32),
            stderr: Vec::new(),
            stdout: stdout.as_bytes().to_vec(),
        })
    }

    #[test]
    fn init_cache_dir_test() {
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };

        let fs = filesystem::MockFileSystem::new();
        fs.is_file.return_value(false);
        let db = init_cache(&fs, &paths).unwrap();
        assert_eq!(
            fs.create_dir_all.calls(),
            vec![
                PathBuf::from("some/random/path/.cracker_storage"),
                PathBuf::from("some/random/path/bin"),
            ]
        );

        assert!(!db.storage_owned_by.is_empty())
    }

    #[test]
    fn init_cache_dir_db_already_exists() {
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };

        let fs = filesystem::MockFileSystem::new();
        fs.is_file.return_value(true);

        let username = whoami::username();

        fs.read_file.return_value(Ok(String::from(format!(
            r#"{{"wrapped":[],"storage_owned_by":"{}"}}"#,
            username
        ))
        .as_bytes()
        .to_vec()));
        let db = init_cache(&fs, &paths).unwrap();
        assert_eq!(
            fs.create_dir_all.calls(),
            vec![PathBuf::from("some/random/path/bin"),]
        );

        assert!(!db.storage_owned_by.is_empty());
    }

    #[test]
    fn init_cache_dir_db_already_exists_diff_username() {
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };

        let fs = filesystem::MockFileSystem::new();
        fs.is_file.return_value(true);

        let username = whoami::username();

        fs.read_file.return_value(Ok(String::from(
            r#"{"wrapped":[],"storage_owned_by":"not_me"}"#,
        )
        .as_bytes()
        .to_vec()));
        let result = init_cache(&fs, &paths);
        let display = format!("{}", result.err().unwrap());
        assert_eq!(
            display,
            r#"Cracker storage owned by: 'not_me' while you are: 'fulara'"#
        );
        assert!(fs.create_dir_all.calls().is_empty());
    }

    #[test]
    fn crack_tests() {
        let req = CrackRequest {
            bin_name: String::from("binary"),
            pkg: ConanPackage::new("abc/321@a/b").unwrap(),
            settings: vec![],
            options: vec![],
        };
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };

        let fs = filesystem::MockFileSystem::new();

        let mut db = CrackerDatabase {
            wrapped: vec![],
            storage_owned_by: String::new(),
        };
        assert!(db.wrapped(&req.bin_name).is_none());
        let result = crack(BufReader::new("".as_bytes()), &fs, &req, &paths, &mut db);
        assert_eq!(
            db.wrapped(&req.bin_name),
            Some(&Wrapper {
                wrapped_bin: String::from("binary")
            })
        );
        let f = &fs.write_file.calls()[0];
        assert_eq!(f.0, PathBuf::from("some/random/path/bin/binary"));
        assert_eq!(
            std::str::from_utf8(&f.1).unwrap(),
            r#"#!/bin/bash
source some/random/path/bin/activate_run.sh
source some/random/path/bin/activate.sh
binary "${@}""#
        );

        let fs = filesystem::MockFileSystem::new();
        let result = crack(BufReader::new("y".as_bytes()), &fs, &req, &paths, &mut db);
        assert_eq!(
            fs.remove_file.calls()[0],
            PathBuf::from("some/random/path/bin/binary")
        );
        let f = &fs.write_file.calls()[0];
        assert_eq!(f.0, PathBuf::from("some/random/path/bin/binary"));
        assert_eq!(
            std::str::from_utf8(&f.1).unwrap(),
            r#"#!/bin/bash
source some/random/path/bin/activate_run.sh
source some/random/path/bin/activate.sh
binary "${@}""#
        );

        // binary wrapped already - user does not want to override.
        let fs = filesystem::MockFileSystem::new();
        crack(BufReader::new("n".as_bytes()), &fs, &req, &paths, &mut db).unwrap();
        assert!(fs.remove_file.calls().is_empty());
        assert!(fs.write_file.calls().is_empty());
    }

    #[test]
    fn conan_install_fun() {
        let mut expected_invocations = vec![
            String::from(r#""conan" "config" "get" "storage.path""#),
            String::from(
                r#""conan" "config" "set" "storage.path" "some/random/path/.cracker_storage""#,
            ),
            String::from(
                r#""conan" "install" "abc/321@" "-if" "some_folder" "-g" "virtualrunenv" "-g" "virtualenv" "-s" "some_set" "-s" "another_one" "-o" "opt""#,
            ),
            String::from(r#""conan" "config" "set" "storage.path" "abc""#),
        ];

        let (sender, receiver) = std::sync::mpsc::channel();

        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };

        Conan::new(|c| assert_command_generate_output(c, sender.clone(), "abc"))
            .unwrap()
            .install(
                &ConanPackage::new("abc/321@").unwrap(),
                &paths,
                "some_folder",
                vec![String::from("some_set"), String::from("another_one")],
                vec![String::from("opt")],
            );
        let captured_invocations: Vec<String> = receiver.try_iter().collect();
        assert_eq!(captured_invocations, expected_invocations);
    }

    #[test]
    fn permissions() {
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
        };
        let mut fs = filesystem::MockFileSystem::new();
        generate_enable_script(&mut fs, &paths).unwrap();

        let call = &fs.write_file.calls()[0];
        assert_eq!(call.0, PathBuf::from("some/random/path/cracker_enable"));

        assert_eq!(
            std::str::from_utf8(&call.1).unwrap(),
            r#"#!/bin/bash
export PATH="some/random/path/bin:$PATH""#
        )
        // let f = std::fs::Permissions::
    }
    #[test]
    fn extract_test_path() {
        let mut fs = filesystem::MockFileSystem::new();
        fs.read_file.return_value(Ok(String::from(r#"
abcabcabc
PATH="wole":"abc"
        "#).into_bytes()));

        assert_eq!(extract_path(&fs, PathBuf::new()), Some(String::from("wole")));
    }


    #[test]
    fn expand_mode_to_all_users_test() {
        assert_eq!(expand_mode_to_all_users(0o100u32), 0o111);
        assert_eq!(expand_mode_to_all_users(0o300u32), 0o333);
        assert_eq!(expand_mode_to_all_users(0o644u32), 0o666);
        assert_eq!(expand_mode_to_all_users(0o713u32), 0o777);
        assert_eq!(expand_mode_to_all_users(0o134u32), 0o135);
    }
}
