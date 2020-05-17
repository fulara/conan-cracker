error_chain! {
    foreign_links {
        Io(::std::io::Error) #[cfg(unix)];
        SerdeJson(::serde_json::error::Error);
    }

    errors {
        ConanNotInPath {
            display("Conan was not found in your path.")
        }
        CrackerStorageDifferentUsername(owned_by: String, called_by : String) {
            description("Cracker storage owned by different user")
            display("Cracker storage owned by: '{}' while you are: '{}'", owned_by, called_by)
        }
    }
}
