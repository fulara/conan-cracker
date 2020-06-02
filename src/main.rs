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
use filesystem::FileSystem;
use std::fs::File;
use walkdir::{DirEntry, Error};
/*
whats next:
stream output while invoking commands.
handle errors from git.

 */

fn info(text: &str) {
    println!("{}", text);
}

fn warn(text: &str) {
    println!("{}", text);
}

fn error(text: &str) {
    println!("{}", text);
}

#[derive(StructOpt, Debug)]
#[structopt(name = "cracker")]
struct Opt {
    #[structopt(subcommand)]
    command: CrackerCommand,
}

#[derive(StructOpt, Debug)]
enum CrackerCommand {
    Install(OptInstall),
    /// Same as 'install'
    Conan(OptInstall),

    Git(OptGit),

    Import(OptImport),
}

#[derive(StructOpt, Debug)]
struct OptInstall {
    #[structopt(long, env = "CRACKER_STORAGE_DIR")]
    prefix: PathBuf,
    #[structopt(long, env = "CRACKER_STORAGE_BIN")]
    bin_dir: Option<PathBuf>,
    reference: String,
    #[structopt(long)]
    wrappers: Vec<String>,
    #[structopt(long, short)]
    settings: Vec<String>,
    #[structopt(long, short)]
    options: Vec<String>,
}

#[derive(StructOpt, Debug)]
struct OptGit {
    #[structopt(long, env = "CRACKER_STORAGE_DIR")]
    prefix: PathBuf,
    #[structopt(long, env = "CRACKER_STORAGE_BIN")]
    bin_dir: Option<PathBuf>,
    url: String,
    #[structopt(long)]
    wrappers: Vec<String>,

    #[structopt(long, default_value = ".")]
    search_paths: Vec<String>,
}

#[derive(StructOpt, Debug)]
struct OptImport {
    #[structopt(long)]
    prefix: PathBuf,
    #[structopt(long)]
    bin_dir: Option<PathBuf>,

    db_path: PathBuf,
}

struct Paths {
    prefix: PathBuf,
    bin_dir: PathBuf,
    install_type: InstallationType,
    pkg_name: String,
}

enum InstallationType {
    Conan,
    Git,
}

impl InstallationType {
    fn format(&self) -> &str {
        match *self {
            InstallationType::Conan => "conan",
            InstallationType::Git => "git",
        }
    }
}

impl Paths {
    pub fn new(
        prefix: PathBuf,
        bin_dir: Option<PathBuf>,
        install_type: InstallationType,
        pkg_name: &str,
    ) -> Self {
        Self {
            bin_dir: bin_dir.unwrap_or(prefix.join("bin")),
            prefix,
            install_type,
            pkg_name: pkg_name.to_owned(),
        }
    }

    fn bin_dir(&self) -> PathBuf {
        self.bin_dir.clone()
    }

    fn storage_dir(&self) -> PathBuf {
        self.prefix.join(".cracker_storage")
    }

    fn db_path(&self) -> PathBuf {
        self.prefix.join(".cracker_index")
    }

    fn install_folder(&self) -> PathBuf {
        //so.. it would be better to actully randomize this - but for now its okayish.
        //but it doesnt handle at all installing mulple version of the packages.
        self.storage_dir()
            .join(format!("{}_{}", self.install_type.format(), self.pkg_name))
    }
}
fn execute(mut c: Command) -> std::io::Result<std::process::Output> {
    info(&format!("now invoking: {:?}", c));
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
        c.args(&["config", "set", &format!("storage.path={}", path)]);
        let c = executor(c).expect(&format!("Unable to set storage path!"));
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
        settings: &[String],
        options: &[String],
    ) -> err::Result<()> {
        info(&format!("Installing package: {}", conan_pkg.full()));
        let guard = ConanStorageGuard::new(self.executor.clone(), &paths.storage_dir().join(".conan"));
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

        let output = (self.executor)(c)?;

        if !output.status.success() {
            Err(err::ErrorKind::ConanInstallFailure(output).into())
        } else {
            Ok(())
        }
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
        let ans = ans.trim();

        if ans == "y" || ans == "yes" {
            return true;
        } else if ans == "n" || ans == "no" {
            return false;
        } else {
            println!(
                "only [y|yes|n|no] is accepted as an answer. you gave: {}",
                ans
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Wrapper {
    wrapped_bin: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum CrackerDatabaseData {
    Conan {
        conan_pkg: ConanPackage,
        conan_settings: Vec<String>,
        conan_options: Vec<String>,
    },
    Git {
        pkg_name: String,
        url: String,
        label: String,
        search_paths: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CrackerDatabaseEntry {
    data: CrackerDatabaseData,
    wrappers: Vec<Wrapper>,
    install_folder: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        let mut sanitized = self.clone();
        sanitized.wrapped.retain(|e| !e.wrappers.is_empty());

        let ser = serde_json::to_string_pretty(&sanitized)?;
        use std::fs;
        Ok(File::create(path)?.write_all(ser.as_bytes())?)
    }

    fn wrapped(&self, wrapper_name: &str) -> Option<(Wrapper)> {
        self.wrapped
            .iter()
            .find_map(|e| e.wrappers.iter().find(|w| &w.wrapped_bin == wrapper_name))
            .cloned()
    }

    fn wrappers(&self, install_folder: &str) -> Vec<Wrapper> {
        self.wrapped
            .iter()
            .filter(|e| e.install_folder == install_folder)
            .map(|e| e.wrappers.clone())
            .next()
            .unwrap_or_default()
    }

    fn register_wrap(&mut self, binary: &str, pkg_dir: &str, data: CrackerDatabaseData) {
        let e_opt = self.wrapped.iter_mut().find(|entry| entry.data == data);
        let e = if let Some(e) = e_opt {
            e
        } else {
            self.wrapped.push(CrackerDatabaseEntry {
                data,
                install_folder: pkg_dir.to_owned(),
                wrappers: vec![],
            });
            self.wrapped.last_mut().unwrap()
        };

        e.wrappers.push(Wrapper {
            wrapped_bin: binary.to_owned(),
        });
    }

    fn unregister_wrapper(&mut self, wrap: &Wrapper) {
        for e in self.wrapped.iter_mut() {
            e.wrappers.retain(|w| w != wrap);
        }
    }

    fn unregister_pkg(&mut self, install_folder: &str) {
        self.wrapped.retain(|f| f.install_folder != install_folder);
    }
}

struct CrackRequest {
    bin: PathBuf,
    pkg_name: String,

    data: CrackerDatabaseData,
}

fn crack<R: Read, Fs: filesystem::FileSystem>(
    reader: BufReader<R>,
    fs: &Fs,
    request: &CrackRequest,
    paths: &Paths,
    db: &mut CrackerDatabase,
    use_conan_wrappers: bool,
) -> std::io::Result<()> {
    let bin_name = request
        .bin
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    info(&format!("Creating wrapper for: {}", bin_name));
    let wrapper_path = paths.bin_dir.join(&bin_name);
    if let Some(wrapper) = db.wrapped(&bin_name) {
        if !input(
            reader,
            &format!("Wrapper {} already generated overwrite?", bin_name),
        ) {
            return Ok(());
        }

        fs.remove_file(&wrapper_path)?;
        db.unregister_wrapper(&wrapper);
    }

    let wrapper_contents = if use_conan_wrappers {
        format!(
            r#"
#!/bin/bash
source {pkg_dir}/activate_run.sh
source {pkg_dir}/activate.sh
{bin_name} "${{@}}"
"#,
            pkg_dir = paths.install_folder().display(),
            bin_name = request.bin.display()
        )
    } else {
        format!(
            r#"
#!/bin/bash
{bin_name} "${{@}}"
"#,
            bin_name = request.bin.display()
        )
    };

    fs.write_file(&wrapper_path, wrapper_contents.trim().to_owned())?;
    if let Ok(metadata) = std::fs::metadata(&wrapper_path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = metadata.permissions();
        let exec_all = perms.mode() | 0o111;
        perms.set_mode(exec_all);
        std::fs::set_permissions(wrapper_path, perms);
    }

    db.register_wrap(
        &bin_name,
        paths.install_folder().to_str().unwrap(),
        request.data.clone(),
    );

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

fn extract_path<Fs: filesystem::FileSystem>(fs: &Fs, path: PathBuf) -> Option<String> {
    let content = fs.read_file(path).ok()?;
    let content = std::str::from_utf8(&content).ok()?;
    for line in content.lines() {
        if line.starts_with("PATH=") {
            let regex =
                regex::Regex::new(r#"^PATH="([^"]+)."#).expect("Path deduction regex was invalid.");
            let captures = regex
                .captures(line)
                .expect("Installed binary didnt have proper PATH?");
            let path = captures
                .get(1)
                .expect("Installed binary didnt have proper PATH?");

            return Some(path.as_str().to_owned());
        }
    }

    None
}

fn preinstall(paths: Paths) -> err::Result<(Paths, CrackerDatabase)> {
    let fs = filesystem::OsFileSystem::new();

    let db = init_cache(&fs, &paths)?;

    generate_enable_script(&fs, &paths);

    Ok((paths, db))
}

fn bump_storage_permission(paths: &Paths) -> err::Result<()> {
    info("now bumping permissions for all the files.");
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

    Ok(())
}

fn make_sure_if_empty<Fs: filesystem::FileSystem>(
    fs: &Fs,
    pkg_name: &str,
    paths: &Paths,
    db: &mut CrackerDatabase,
) -> bool {
    let if_path = paths.install_folder();
    if fs.is_dir(&if_path) {
        let wrappers = db.wrappers(if_path.to_str().unwrap());
        let bins: Vec<String> = wrappers.iter().map(|w| w.wrapped_bin.clone()).collect();
        if input(BufReader::new(std::io::stdin().lock()), &format!("Package: {} already installed wraps: [{}], to proceed that package has to be removed, remove?", pkg_name, bins.join(", "))) {
            if let Err(e) = fs.remove_dir_all(&if_path) {
                warn(&format!("Failure while removing if: {}, continued. {:?}", if_path.display(), e));
            }
            for bin in bins {
                if let Err(_) = fs.remove_file(paths.bin_dir().join(&bin)) {
                    warn(&format!("Failure while removing wrapper: {}, continued.", bin));
                }
            }

            db.unregister_pkg(if_path.to_str().unwrap());
            db.save(paths.db_path());
            info("ok removed.");
            return true;
        } else {
            return false;
        }
    }

    return true;
}

fn crackem<Fs: filesystem::FileSystem>(
    fs: &Fs,
    paths: &Paths,
    db: &mut CrackerDatabase,
    root_path: String,
    pkg_name: &str,
    wrappers: &[String],
    data: CrackerDatabaseData,
    use_conan_wrappers: bool,
) -> err::Result<()> {
    for entry in walkdir::WalkDir::new(root_path).max_depth(1) {
        match entry {
            Ok(entry) => {
                if !entry.file_type().is_file() {
                    continue;
                }
                let p = entry.path();
                use std::os::unix::fs::PermissionsExt;
                if 0o100
                    & std::fs::metadata(p)
                        .expect("unable to extract metadata")
                        .permissions()
                        .mode()
                    != 0
                {
                    if !wrappers.is_empty()
                        && !wrappers.contains(&p.file_name().unwrap().to_str().unwrap().to_string())
                    {
                        continue;
                    }
                    crack(
                        BufReader::new(std::io::stdin().lock()),
                        fs,
                        &CrackRequest {
                            pkg_name: pkg_name.to_owned(),
                            bin: std::fs::canonicalize(p.to_path_buf()).unwrap(),
                            data: data.clone(),
                        },
                        &paths,
                        db,
                        use_conan_wrappers,
                    )?;
                }
            }
            Err(e) => {
                println!("got error while iterating: {}", e);
            }
        }
    }

    Ok(())
}

fn extract_git_repo_name(url: &str) -> err::Result<String> {
    let regex = regex::Regex::new(r"^.*/(.*)\.git$").expect("Invalid git regex.");

    if !regex.is_match(url) {
        return Err(err::ErrorKind::GitUnableToExtractProjectName(url.to_owned()).into());
    }

    Ok(regex
        .captures(url)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .to_owned())
}

fn do_git_install(i: OptGit) -> err::Result<()> {
    let fs = filesystem::OsFileSystem::new();
    let pkg_name = extract_git_repo_name(&i.url)?;
    let paths = Paths::new(i.prefix, i.bin_dir, InstallationType::Git, &pkg_name);
    let (paths, mut db) = preinstall(paths)?;

    if !make_sure_if_empty(&fs, &pkg_name, &paths, &mut db) {
        warn("Unable to install package.");
        return Ok(());
    }

    let mut c = Command::new("git");
    c.args(&[
        "clone",
        &i.url,
        "--depth",
        "1",
        paths.install_folder().as_os_str().to_str().unwrap(),
    ]);
    execute(c);

    for path in i.search_paths.iter() {
        let path = paths
            .install_folder()
            .join(path)
            .to_str()
            .unwrap()
            .to_string();
        crackem(
            &fs,
            &paths,
            &mut db,
            path.to_string(),
            &pkg_name,
            &i.wrappers,
            CrackerDatabaseData::Git {
                pkg_name: pkg_name.clone(),
                url: i.url.clone(),
                label: "unimplemented".to_string(),
                search_paths: i.search_paths.clone(),
            },
            false,
        )?;
    }

    bump_storage_permission(&paths);
    db.save(paths.db_path());
    Ok(())
}

fn do_install(i: OptInstall) -> err::Result<()> {
    let fs = filesystem::OsFileSystem::new();
    let conan_pkg = ConanPackage::new(&i.reference)?;
    let paths = Paths::new(
        i.prefix,
        i.bin_dir,
        InstallationType::Conan,
        &conan_pkg.name,
    );
    let (paths, mut db) = preinstall(paths)?;

    let conan = Conan::new(execute)?;

    let if_path = paths.install_folder();
    if !make_sure_if_empty(&fs, &conan_pkg.name, &paths, &mut db) {
        warn("Unable to install package.");
        return Ok(());
    }

    let install_folder = if_path
        .as_os_str()
        .to_str()
        .ok_or("unable to generate if folder")?;
    conan.install(&conan_pkg, &paths, &install_folder, &i.settings, &i.options)?;

    let env_run_path = if_path.join("environment_run.sh.env");
    let path = extract_path(&fs, env_run_path).expect(
        "environment_run.sh.env did not contain correct PATH? non binary package requested?",
    );

    crackem(
        &fs,
        &paths,
        &mut db,
        path,
        &conan_pkg.name,
        &i.wrappers,
        CrackerDatabaseData::Conan {
            conan_pkg: conan_pkg.clone(),
            conan_settings: i.settings.clone(),
            conan_options: i.options.clone(),
        },
        true,
    )?;

    bump_storage_permission(&paths);
    db.save(paths.db_path());

    Ok(())
}

fn do_import(i: OptImport) -> err::Result<()> {
    let fs = filesystem::OsFileSystem::new();
    let db = CrackerDatabase::load(&fs, i.db_path)?;

    for wrapped in db.wrapped {
        let wrappers = wrapped
            .wrappers
            .iter()
            .map(|w| w.wrapped_bin.clone())
            .collect();
        match wrapped.data {
            CrackerDatabaseData::Conan {
                conan_pkg,
                conan_options,
                conan_settings,
            } => {
                info(&format!("now installing: {}", conan_pkg.full()));
                let install = OptInstall {
                    prefix: i.prefix.clone(),
                    bin_dir: i.bin_dir.clone(),
                    options: conan_options,
                    settings: conan_settings,
                    wrappers,
                    reference: conan_pkg.full(),
                };

                do_install(install)?;
            }
            CrackerDatabaseData::Git {
                pkg_name,
                url,
                label,
                search_paths,
            } => {
                info(&format!("now installing: {}", url));

                let install = OptGit {
                    url,
                    search_paths,
                    wrappers,
                    prefix: i.prefix.clone(),
                    bin_dir: i.bin_dir.clone(),
                };

                do_git_install(install)?;
            }
        }
    }

    Ok(())
}

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:#?}", opt);

    match opt.command {
        CrackerCommand::Install(i) | CrackerCommand::Conan(i) => {
            if let Err(e) = do_install(i) {
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
                    err::ErrorKind::ConanInstallFailure(_) => {
                        println!("{}", e);
                    }
                    err::ErrorKind::__Nonexhaustive {} => {}
                    err::ErrorKind::GitUnableToExtractProjectName(_) => {
                        println!("{}", e);
                    }
                }
            }
        }
        CrackerCommand::Import(i) => {
            do_import(i);
        }
        CrackerCommand::Git(i) => {
            do_git_install(i);
        }
    }
}

#[cfg(test)]
mod package_tests {
    use crate::conan_package::ConanPackage;
    use crate::{
        crack, err, expand_mode_to_all_users, extract_git_repo_name, extract_path,
        generate_enable_script, init_cache, Conan, CrackRequest, CrackerDatabase,
        CrackerDatabaseData, CrackerDatabaseEntry, InstallationType, Paths, Wrapper,
    };
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
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
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
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
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
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
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
            bin: PathBuf::from("binary"),
            pkg_name: String::from("abc"),
            data: CrackerDatabaseData::Conan {
                conan_pkg: ConanPackage::new("abc/321@a/b").unwrap(),
                conan_settings: vec![],
                conan_options: vec![],
            },
        };
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
        };

        let fs = filesystem::MockFileSystem::new();

        let mut db = CrackerDatabase {
            wrapped: vec![],
            storage_owned_by: String::new(),
        };
        assert!(db
            .wrapped(req.bin.file_name().unwrap().to_str().unwrap())
            .is_none());
        let result = crack(
            BufReader::new("".as_bytes()),
            &fs,
            &req,
            &paths,
            &mut db,
            true,
        );
        assert_eq!(
            db.wrapped(req.bin.file_name().unwrap().to_str().unwrap()),
            Some(Wrapper {
                wrapped_bin: String::from("binary")
            })
        );
        let f = &fs.write_file.calls()[0];
        assert_eq!(f.0, PathBuf::from("some/random/path/bin/binary"));
        assert_eq!(
            std::str::from_utf8(&f.1).unwrap(),
            r#"#!/bin/bash
source some/random/path/.cracker_storage/conan_abc/activate_run.sh
source some/random/path/.cracker_storage/conan_abc/activate.sh
binary "${@}""#
        );

        let fs = filesystem::MockFileSystem::new();
        let result = crack(
            BufReader::new("y".as_bytes()),
            &fs,
            &req,
            &paths,
            &mut db,
            true,
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
source some/random/path/.cracker_storage/conan_abc/activate_run.sh
source some/random/path/.cracker_storage/conan_abc/activate.sh
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
            true,
        )
        .unwrap();
        assert!(fs.remove_file.calls().is_empty());
        assert!(fs.write_file.calls().is_empty());
    }

    #[test]
    fn conan_install_fun() {
        let mut expected_invocations = vec![
            String::from(r#""conan" "config" "get" "storage.path""#),
            String::from(
                r#""conan" "config" "set" "storage.path=some/random/path/.cracker_storage""#,
            ),
            String::from(
                r#""conan" "install" "abc/321@" "-if" "some_folder" "-g" "virtualrunenv" "-g" "virtualenv" "-s" "some_set" "-s" "another_one" "-o" "opt""#,
            ),
            String::from(r#""conan" "config" "set" "storage.path=abc""#),
        ];

        let (sender, receiver) = std::sync::mpsc::channel();

        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
        };

        Conan::new(|c| assert_command_generate_output(c, sender.clone(), "abc"))
            .unwrap()
            .install(
                &ConanPackage::new("abc/321@").unwrap(),
                &paths,
                "some_folder",
                &vec![String::from("some_set"), String::from("another_one")],
                &vec![String::from("opt")],
            );
        let captured_invocations: Vec<String> = receiver.try_iter().collect();
        assert_eq!(captured_invocations, expected_invocations);
    }

    #[test]
    fn permissions() {
        let paths = Paths {
            prefix: PathBuf::from("some/random/path"),
            bin_dir: PathBuf::from("some/random/path/bin"),
            install_type: InstallationType::Conan,
            pkg_name: "abc".to_owned(),
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
        fs.read_file.return_value(Ok(String::from(
            r#"
abcabcabc
PATH="wole":"abc"
        "#,
        )
        .into_bytes()));

        assert_eq!(
            extract_path(&fs, PathBuf::new()),
            Some(String::from("wole"))
        );
    }

    #[test]
    fn expand_mode_to_all_users_test() {
        assert_eq!(expand_mode_to_all_users(0o100u32), 0o111);
        assert_eq!(expand_mode_to_all_users(0o300u32), 0o333);
        assert_eq!(expand_mode_to_all_users(0o644u32), 0o666);
        assert_eq!(expand_mode_to_all_users(0o713u32), 0o777);
        assert_eq!(expand_mode_to_all_users(0o134u32), 0o135);
    }

    #[test]
    fn extract_git_url_project_name() {
        assert_eq!(
            extract_git_repo_name("https://github.com/fulara/conan-cracker.git").unwrap(),
            String::from("conan-cracker")
        );
    }
}
