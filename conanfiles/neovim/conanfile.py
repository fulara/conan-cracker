import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class NeovimConan(ConanFile):
    settings = "os", "arch"
    name="neovim"
    #version = "0.4.3"
   
    build_requires = (
        "cmake/[>=3.16.6]",
        "libtool/[>=2.4.6]",
        "automake/[>=1.16.1]",
        "autoconf/[>=2.69]",
        "m4/[>=1.4.18]",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)
   
    def source(self): 
        tools.get("https://github.com/neovim/neovim/archive/v{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        
        
    def build(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.make(args=["CMAKE_BUILD_TYPE=RelWithDebInfo", "CMAKE_INSTALL_PREFIX={}".format(self.package_folder)])

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()
