use std::path::{PathBuf, Path};
use std::borrow::Borrow;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name="cracker")]
struct Opt {
    #[structopt(short, long)]
    install: String,

    #[structopt(short, long)]
    wrappers : Vec<String>,

    #[structopt(short, long)]
    prefix : Option<PathBuf>,

    #[structopt(short, long)]
    bin_dir : Option<PathBuf>,
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

#[derive(Debug, PartialEq, Clone)]
struct ConanPackage {
    name : String,
    version : String,
    user : String,
    channel : String,
}

impl ConanPackage {
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

impl ConanPackage {
    fn new(reference: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // todo: add proper checks here.
        // if reference.contains("@") {
        //     if reference.matches("/").count() != 2 {
        //         Err("Your conan package does not contain '/' name/version format is required.")?
        //     }
        // }

        let package = if reference.contains("@") {
            reference.to_owned()
        } else {
            format!("{}@", reference)
        };

        if package.len() <= 5 {
            Err(format!("conan package provided({}) is too short, conan does not handle that 5+ charachters only.", package))?
        }

        let mut full_name_split = package.split('@');
        let name_version = full_name_split.next().unwrap();

        let user_channel = full_name_split.next().unwrap();
        let (user, channel) = if user_channel.len() == 0 {
            (String::new(), String::new())
        } else {
            let user_channel : Vec<_> = user_channel.split("/").collect();
            if user_channel.len() == 2 {
                (user_channel[0].to_owned(), user_channel[1].to_owned())
            } else {
                Err(format!("Missing chanel in your ref: {}", reference))?
            }
        };

        let mut name_version_split = name_version.split("/");
        Ok(ConanPackage {
            name : name_version_split.next().unwrap().to_owned(),
            version: name_version_split.next().unwrap().to_owned(),
            user,
            channel,
        })
    }
}

fn init_cache(paths: &Paths) {
    std::fs::create_dir_all(&paths.storage_dir()).expect(&format!("Unable to create a prefix dir: {}", paths.prefix.display()));
    std::fs::create_dir_all(&paths.bin_dir).expect(&format!("Unable to create a bib dir: {}", paths.bin_dir.display()));
}



fn main() {
    let opt : Opt= Opt::from_args();
    println!("{:#?}", opt);

    let env_path = std::env::var("CRACKER_STORAGE_DIR").ok().map(|p| PathBuf::from(p));

    let prefix =
        opt.prefix.or(env_path).expect("provide either prefix or define CRACKER_STORAGE_DIR env.");

    let mut bin_dir = prefix.clone();
    bin_dir.push("bin");
    let bin_dir_env = std::env::var("CRACKER_STORAGE_BIN").ok().map(|p| PathBuf::from(p));
    let bin_dir = opt.bin_dir.or(bin_dir_env).or(Some(bin_dir)).unwrap();

    let paths = Paths {
        prefix,
        bin_dir
    };

    init_cache(&paths);

}

#[cfg(test)]
mod package_tests {
    use crate::ConanPackage;
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
        assert_eq!("abc/321@a/b", ConanPackage::new("abc/321").unwrap().full());
    }
}