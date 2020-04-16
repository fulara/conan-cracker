import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "apr"
    version = "1.7.0"
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)

    def source(self): 
        tools.get("https://downloads.apache.org//apr/apr-{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        

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
       self.cpp_info.libs = tools.collect_libs(self)
