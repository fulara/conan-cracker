import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "git"
    version = "2.26.1"
    requires = "openssl/1.1.1d",
    build_requires = (
        "autoconf/2.69",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"


    def source(self): 
        tools.get("https://github.com/git/git/archive/v{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)

    def build(self):
        re = RunEnvironment(self)
        be = AutoToolsBuildEnvironment(self)
        be.link_flags.extend(["-pthread", "-ldl"])
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(re.vars):
                with tools.environment_append(be.vars):
                    print (os.environ)
                    self.run("make configure")
                    be.configure()
                    be.make()

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()

    def package_info(self):
        self.cpp_info.system_libs.extend(["pthread", "rt", "dl"])
