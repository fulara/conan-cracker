
#[derive(Debug, PartialEq, Clone)]
pub struct ConanPackage {
    name : String,
    version : String,
    user : String,
    channel : String,
}

impl ConanPackage {
    pub fn new(reference: &str) -> Result<Self, Box<dyn std::error::Error>> {
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

    pub fn full(&self) -> String {
        format!("{}/{}@{}", self.name, self.version, self.user_channel())
    }

    pub fn user_channel(&self) -> String {
        if self.user.is_empty() {
            String::new()
        } else {
            format!("{}/{}", self.user, self.channel)
        }
    }
}


#[cfg(test)]
mod conan_package_tests {
    use super::*;
    fn p(name : &str, ver : &str, user : &str, channel: &str) -> ConanPackage {
        ConanPackage {
            name : name.to_owned(),
            version : ver.to_owned(),
            user : user.to_owned(),
            channel : channel.to_owned(),


        }
    }
    fn name_pattern_fail_test(package : &str) {
        assert_eq!(ConanPackage::new(package).err().unwrap().to_string(), format!("Your reference does not match a regex pattern, {}", package));
    }

    fn name_pattern_ok(package : &str) {
        assert!(ConanPackage::new(package).is_ok());
    }

    #[test]
    fn conan_package_component_extractions() {
        let pkg = p("abc", "321", "", "" );
        assert_eq!(pkg, ConanPackage::new("abc/321").unwrap());
        assert_eq!("abc/321@", ConanPackage::new("abc/321").unwrap().full());
        let pkg = p("abc", "321", "a", "b" );
        assert_eq!(pkg, ConanPackage::new("abc/321@a/b").unwrap());
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
}