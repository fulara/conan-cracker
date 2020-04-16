import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "apr-util"
    version = "1.6.1"
 
    requires = (
        "apr/1.7.0",
        "expat/2.2.9", 
        "sqlite3/3.31.1",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)

    def source(self): 
        tools.get("https://downloads.apache.org//apr/apr-util-{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)

    def build(self):
        apr_path = self.deps_cpp_info["apr"].rootpath
        expat_path = self.deps_cpp_info["expat"].rootpath
        sqlite3_path = self.deps_cpp_info["sqlite3"].rootpath
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.configure(args= ["--with-apr={}".format(apr_path), "--with-expat={}".format(expat_path), "--with-sqlite3={}".format(sqlite3_path)] )
                    be.make()

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()

