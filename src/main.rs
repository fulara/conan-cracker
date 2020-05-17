use std::path::PathBuf;

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Output};
use structopt::StructOpt;

use serde::{Deserialize, Serialize};

#[macro_use]
extern crate error_chain;

mod conan_package;
use crate::conan_package::*;
/*
whats next:
build the db - somehwat done, whats left is saving it to cache.
invoke conan commands.
conan commands should be wrapped with setting of storage.
verify user
 */

mod err {
    error_chain! {
        foreign_links {
            Fmt(::std::fmt::Error);
            Io(::std::io::Error) #[cfg(unix)];
            SerdeJson(::serde_json::error::Error);
        }

        errors {
            CrackerStorageDifferentUsername(owned_by: String, called_by : String) {
                description("Cracker storage owned by different user")
                display("Cracker storage owned by: '{}' while you are: '{}'", owned_by, called_by)
            }
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "cracker")]
enum Opt {
    Install {
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
    },
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
}
fn execute(mut c: Command) -> std::io::Result<std::process::Output> {
    c.output()
}

struct Conan {
    executor: Box<dyn Fn(Command) -> std::io::Result<std::process::Output>>,
}

impl Conan {
    fn new<F: 'static + Fn(Command) -> std::io::Result<Output>>(executor: F) -> Self {
        Self {
            executor: Box::new(executor),
        }
    }

    fn install(
        &self,
        conan_pkg: &ConanPackage,
        install_folder: &str,
        settings: Vec<String>,
        options: Vec<String>,
    ) -> std::io::Result<Output> {
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
        (self.executor)(c)
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
    fn wrapped(&self, wrapper_name: &str) -> Option<(&Wrapper)> {
        self.wrapped
            .iter()
            .find_map(|e| e.wrappers.iter().find(|w| &w.wrapped_bin == wrapper_name))
    }

    fn register_wrap(&mut self, binary: &str, req: &CrackRequest) {
        let e_opt = self
            .wrapped
            .iter_mut()
            .find(|entry| {
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
                conan_settings : req.options.to_vec(),
                conan_options : req.settings.to_vec(),
            });
            self.wrapped.last_mut().unwrap()
        };

        e.wrappers.push(Wrapper {
            wrapped_bin: binary.to_owned(),
        });
    }
}

struct CrackRequest {
    pkg : ConanPackage,
    bin_name: String,
    settings : Vec<String>,
    options : Vec<String>,
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

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:#?}", opt);

    let env_path = std::env::var("CRACKER_STORAGE_DIR")
        .ok()
        .map(|p| PathBuf::from(p));

    match opt {
        Opt::Install {
            reference,
            bin_dir,
            prefix,
            generate_enable,
            wrappers,
            settings,
            options,
        } => {
            let prefix = prefix
                .or(env_path)
                .expect("provide either prefix or define CRACKER_STORAGE_DIR env.");

            let mut selected_bin_dir = prefix.clone();
            selected_bin_dir.push("bin");
            let bin_dir_env = std::env::var("CRACKER_STORAGE_BIN")
                .ok()
                .map(|p| PathBuf::from(p));
            let bin_dir = bin_dir.or(bin_dir_env).or(Some(selected_bin_dir)).unwrap();

            let fs = filesystem::OsFileSystem::new();
            let paths = Paths { prefix, bin_dir };

            init_cache(&fs, &paths);

            if generate_enable {
                generate_enable_script(&fs, &paths);
            }
        }
    }
}

#[cfg(test)]
mod package_tests {
    use crate::conan_package::ConanPackage;
    use crate::{
        crack, err, generate_enable_script, init_cache, Conan, CrackRequest, CrackerDatabase,
        CrackerDatabaseEntry, Paths, Wrapper,
    };
    use std::collections::BTreeMap;
    use std::io::BufReader;
    use std::path::PathBuf;
    use std::process::Command;

    fn assert_command_generate_output(
        c: Command,
        expected_invocation: &str,
        stdout: &str,
    ) -> std::io::Result<std::process::Output> {
        let invocation = format!("{:?}", c);
        assert_eq!(expected_invocation, invocation);

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
            pkg:  ConanPackage::new("abc/321@a/b").unwrap(),
            settings : vec![],
            options : vec![],
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
        let result = crack(
            BufReader::new("".as_bytes()),
            &fs,
            &req,
            &paths,
            &mut db,
        );
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
        let result = crack(
            BufReader::new("y".as_bytes()),
            &fs,
            &req,
            &paths,
            &mut db,
        );
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
        crack(
            BufReader::new("n".as_bytes()),
            &fs,
            &req,
            &paths,
            &mut db,
        )
        .unwrap();
        assert!(fs.remove_file.calls().is_empty());
        assert!(fs.write_file.calls().is_empty());
    }

    #[test]
    fn conan_install_fun() {
        Conan::new(|c| assert_command_generate_output(
            c,
            r#""conan" "install" "abc/321@" "-if" "some_folder" "-g" "virtualrunenv" "-g" "virtualenv" "-s" "some_set" "-s" "another_one" "-o" "opt""#,
            "abc"))
            .install(
                &ConanPackage::new("abc/321@").unwrap(),
                "some_folder",
                vec![String::from("some_set"), String::from("another_one")],
                vec![String::from("opt")],
            );
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
}
