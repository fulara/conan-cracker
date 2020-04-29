import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment


class GitConan(ConanFile):
    settings = "os_build", "arch"
    name = "m4"
    version = "1.4.18"
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"
    
    def source(self): 
        tools.get("https://github.com/tar-mirror/gnu-m4/archive/v{}.tar.gz".format(self.version))
        os.rename("gnu-m4-{}".format(self.version), self._source_subfolder)
        

    def build(self):
        be = AutoToolsBuildEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                be.configure()
                be.make()

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                be.install()
           
    def package_info(self):
        self.env_info.M4.append(os.path.join(self.package_folder, "bin", "m4"))
