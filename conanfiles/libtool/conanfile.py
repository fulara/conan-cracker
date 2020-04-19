import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "libtool"
    version = "2.4.6"
   
    build_requires = (
        "m4/1.4.18",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)

    def source(self): 
        tools.get("https://ftp.gnu.org/gnu/libtool/libtool-{}.tar.xz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        
    def build(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.configure()
                    be.make()

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()

