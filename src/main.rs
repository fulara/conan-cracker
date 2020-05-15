use std::path::{PathBuf, Path};
use std::borrow::Borrow;

use structopt::StructOpt;
use std::io::{Write, Read, BufRead, BufReader};
use std::collections::{BTreeMap, HashSet, HashMap};
use std::process::Output;

/*
whats next:
build the db
invoke conan commands.
 */

#[derive(StructOpt, Debug)]
#[structopt(name="cracker")]
enum Opt {
    Install {
        #[structopt(long)]
        wrappers : Vec<String>,

        #[structopt(long)]
        prefix : Option<PathBuf>,

        #[structopt(long)]
        bin_dir : Option<PathBuf>,

        #[structopt(long)]
        generate_enable : bool,
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum Action {
    CreateFile { filename : PathBuf, content : String },
    RemoveFile { filename : PathBuf,},
    CreateDir { path: PathBuf },
}

struct Paths {
    prefix : PathBuf,
    bin_dir : PathBuf,
}

impl Paths {
    fn storage_dir(&self) -> PathBuf {
        self.prefix.join(".cracker_storage")
    }
}
fn execute(mut c : std::process::Command ) -> std::io::Result<std::process::Output> {
    c.output()
}

#[derive()]
struct Conan {
    executor : Box<dyn Fn() -> std::io::Result<std::process::Output>>,
}

impl Conan {
    fn new<F : 'static + Fn() -> std::io::Result<std::process::Output>>(executor : F) -> Self {
        Self {
            executor : Box::new(executor)
        }
    }
    fn install<F : Fn() -> std::io::Result<std::process::Output>>(executor : F) {
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ConanPackage {
    name : String,
    version : String,
    user : String,
    channel : String,
}

impl ConanPackage {
    fn new(reference: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let regex = regex::Regex::new(r"^([^/@]+)[/]([^/@]+)(@?$|@([^/@]+)[/]([^/@]+)$)")?;
        if ! regex.is_match(reference) {
            Err(format!("Your reference does not match a regex pattern, {}", reference))?
        }

        if reference.len() <= 5 {
            Err(format!("conan package provided({}) is too short, conan does not handle that 5+ charachters only.", reference))?
        }
        let captures = regex.captures(reference).unwrap();

        let name = captures.get(1).unwrap().as_str().to_owned();
        let version = captures.get(2).unwrap().as_str().to_owned();
        let user = captures.get(4).map_or("", |m| m.as_str()).to_owned();
        let channel = captures.get(5).map_or("", |m| m.as_str()).to_owned();

        Ok(ConanPackage {
            name,version,user,channel,
        })
    }

    fn full(&self) -> String {
        format!("{}/{}@{}", self.name, self.version, self.user_channel())
    }

    fn user_channel(&self) -> String {
        if self.user.is_empty() {
            String::new()
        } else {
            format!("{}/{}", self.user, self.channel)
        }
    }
}

fn init_cache(paths: &Paths) -> Vec<Action> {
    vec![
        Action::CreateDir { path : paths.storage_dir()},
        Action::CreateDir { path : paths.bin_dir.clone() },
    ]
    // std::fs::create_dir_all(&paths.storage_dir()).expect(&format!("Unable to create a prefix dir: {}", paths.prefix.display()));
    // std::fs::create_dir_all(&paths.bin_dir).expect(&format!("Unable to create a bib dir: {}", paths.bin_dir.display()));
}

fn generate_enable_script(paths: &Paths) -> Result<Action, Box<dyn std::error::Error>> {
    let path = paths.bin_dir.parent().ok_or("Unable to extract parent path")?.join("cracker_enable");
    // let mut file = std::fs::File::create(path)?;

    // file.write_all(
    Ok(Action::CreateFile {content : format!(r#"
#!/bin/bash
export PATH="{}:$PATH"
"#, paths.bin_dir.display()).trim().to_owned(), filename : path })
}

fn input<R: Read> (mut reader: BufReader<R>, message: &'_ impl std::fmt::Display) -> bool
{
    loop {
        println!("{}", message);
        let mut ans = String::new();
        reader.read_line(&mut ans).expect("Failed to read from stdin");

        let ans = ans.to_lowercase();

        if &ans == "y" || &ans == "yes" {
            return true;
        } else if &ans == "n" || &ans == "no" {
            return false
        } else {
            println!("only [y|yes|n|no] is accepted as an answer.")
        }
    }
}

struct Wrapper {
    wrapped_bin : String,
    used_name : String,
}

struct CrackerDatabaseEntry {
    conan_pkg : ConanPackage,
    wrappers : Vec<Wrapper>,
}

struct CrackerDatabase {
    wrapped : Vec<CrackerDatabaseEntry>,
}

impl CrackerDatabase {
    fn wrapped(&self, wrapper_name : &str) -> Option<(&Wrapper)> {
        self.wrapped.iter().find_map(|e| e.wrappers.iter().find(|w| &w.used_name == wrapper_name))
    }
}

struct CrackRequest {
    bin_name: String,
    wrapper_name : String,
}

fn crack<R: Read>(reader: BufReader<R>, request : &CrackRequest, pkg : &ConanPackage, paths : &Paths, db : &mut CrackerDatabase) -> Vec<Action>  {
    let wrapper_path = paths.bin_dir.join(&request.wrapper_name);
    let mut actions = vec![];
    if let Some(wrapper) = db.wrapped(&request.wrapper_name) {
        if ! input(reader, &format!("Wrapper {} already generated for binary: {} overwrite?", request.wrapper_name, wrapper.wrapped_bin)) {
            return Vec::new();
        }

        actions.push(Action::RemoveFile { filename : wrapper_path.clone() })
    }

    actions.push(Action::CreateFile {content : format!(r#"
#!/bin/bash
source {pkg_dir}/activate_run.sh
source {pkg_dir}/activate.sh
{bin_name} "${{@}}"
"#, pkg_dir=paths.bin_dir.display(), bin_name = request.bin_name).trim().to_owned(), filename : wrapper_path });

    actions
}

fn main() {
    let opt : Opt= Opt::from_args();
    println!("{:#?}", opt);

    let env_path = std::env::var("CRACKER_STORAGE_DIR").ok().map(|p| PathBuf::from(p));

    match opt {
        Opt::Install {bin_dir, prefix, generate_enable, wrappers} => {
            let prefix =
                prefix.or(env_path).expect("provide either prefix or define CRACKER_STORAGE_DIR env.");

            let mut selected_bin_dir = prefix.clone();
            selected_bin_dir.push("bin");
            let bin_dir_env = std::env::var("CRACKER_STORAGE_BIN").ok().map(|p| PathBuf::from(p));
            let bin_dir = bin_dir.or(bin_dir_env).or(Some(selected_bin_dir)).unwrap();

            let paths = Paths {
                prefix,
                bin_dir
            };

            init_cache(&paths);

            if generate_enable {
                generate_enable_script(&paths);
            }
        }
    }
}

#[cfg(test)]
mod package_tests {
    use crate::{ConanPackage, Paths, init_cache, Action, crack, CrackRequest, CrackerDatabase, Conan, CrackerDatabaseEntry, Wrapper};
    use std::path::PathBuf;
    use std::io::BufReader;
    use std::collections::BTreeMap;

    fn p(name : &str, ver : &str, user : &str, channel: &str) -> ConanPackage {
        ConanPackage {
            name : name.to_owned(),
            version : ver.to_owned(),
            user : user.to_owned(),
            channel : channel.to_owned(),


        }
    }
    #[test]
    fn test() {
        let pkg = p("abc", "321", "", "" );
        assert_eq!(pkg, ConanPackage::new("abc/321").unwrap());
        assert_eq!("abc/321@", ConanPackage::new("abc/321").unwrap().full());
        let pkg = p("abc", "321", "a", "b" );
        assert_eq!(pkg, ConanPackage::new("abc/321@a/b").unwrap());
    }

    fn name_pattern_fail_test(package : &str) {
        assert_eq!(ConanPackage::new(package).err().unwrap().to_string(), format!("Your reference does not match a regex pattern, {}", package));
    }

    fn name_pattern_ok(package : &str) {
        assert!(ConanPackage::new(package).is_ok());
    }

    fn generate_output(stdout : &str) -> std::io::Result<std::process::Output> {
        use std::process::{Output, ExitStatus};
        use std::os::unix::process::ExitStatusExt;
        Ok(Output {
            status : ExitStatus::from_raw(0i32),
            stderr : Vec::new(),
            stdout : stdout.as_bytes().to_vec(),
        })
    }

    #[test]
    fn invalid_reference_tests() {
        name_pattern_fail_test("a");
        name_pattern_fail_test("aaaaaa@");
        name_pattern_ok("aaaa/aaaa@");

        //user channel present without slash
        name_pattern_fail_test("aaa/aaa@aa");
        name_pattern_fail_test("aaa/aaa@aaaa");
        name_pattern_ok("aaa/aaa@aaaa/a");

        name_pattern_fail_test("aaa/aaa/aaa");
    }

    #[test]
    fn init_cache_dir_test() {
        let paths = Paths {prefix : PathBuf::from("some/random/path"), bin_dir : PathBuf::from("some/random/path/bin")};

        assert_eq!(init_cache(&paths), vec![
        Action::CreateDir {path : PathBuf::from("some/random/path/.cracker_storage")},
        Action::CreateDir {path : PathBuf::from("some/random/path/bin")}
        ]);
    }

    #[test]
    fn crack_tests() {
        let req = CrackRequest {
            wrapper_name : String::from("wrap"),
            bin_name : String::from("binary"),
        };
        let pkg = ConanPackage::new("abc/321@a/b").unwrap();
        let paths = Paths {prefix : PathBuf::from("some/random/path"), bin_dir : PathBuf::from("some/random/path/bin")};

        let mut db = CrackerDatabase { wrapped : vec![] };
        let result = crack(BufReader::new("".as_bytes()), &req, &pkg, &paths, &mut db);
        assert_eq!(result, vec![
            Action::CreateFile {content : String::from(r#"
#!/bin/bash
source some/random/path/bin/activate_run.sh
source some/random/path/bin/activate.sh
binary "${@}"
            "#).trim().to_owned(), filename  : PathBuf::from("some/random/path/bin/wrap")},
        ]);

        // binary wrapped already - user want to override.
        db.wrapped.push(CrackerDatabaseEntry {
            wrappers : vec![Wrapper { wrapped_bin : String::from("binary"), used_name : String::from("wrap")}],
            conan_pkg : ConanPackage::new("abc/1.0.0@").unwrap(),
        });
        let result = crack(BufReader::new("y".as_bytes()), &req, &pkg, &paths, &mut db);
        assert_eq!(result, vec![
            Action::RemoveFile { filename: PathBuf::from("some/random/path/bin/wrap") },
            Action::CreateFile {content : String::from(r#"
#!/bin/bash
source some/random/path/bin/activate_run.sh
source some/random/path/bin/activate.sh
binary "${@}"
            "#).trim().to_owned(), filename  : PathBuf::from("some/random/path/bin/wrap")},
        ]);

        // binary wrapped already - user does not want to override.
        let result = crack(BufReader::new("n".as_bytes()), &req, &pkg, &paths, &mut db);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn conan_install_fun() {
        Conan::new(|| generate_output("abc"));
    }
}