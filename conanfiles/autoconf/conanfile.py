import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "autoconf"
    version = "2.69"

    build_requires = (
        "m4-tmp/1.4.18",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"
   
    def source(self): 
        tools.get("http://ftp.gnu.org/gnu/autoconf/autoconf-{}.tar.xz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        
    def build(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.configure()
                    self.run("make")

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()
