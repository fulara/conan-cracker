error_chain! {
    foreign_links {
        Io(::std::io::Error) #[cfg(unix)];
        SerdeJson(::serde_json::error::Error);
    }

    errors {
        ConanNotInPath {
            display("Conan was not found in your path.")
        }
        ConanInstallFailure(o: ::std::process::Output) {
            display("conan install failed with {} \nstdout:\n{} \nstderr:\n{}", o.status, std::str::from_utf8(&o.stdout).unwrap(), std::str::from_utf8(&o.stderr).unwrap())
        }
        CrackerStorageDifferentUsername(owned_by: String, called_by : String) {
            description("Cracker storage owned by different user")
            display("Cracker storage owned by: '{}' while you are: '{}'", owned_by, called_by)
        }
    }
}
