import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "lz4"
    version = "1.9.2"
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)

    def source(self): 
        tools.get("https://github.com/lz4/lz4/archive/v{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        

    def build(self):
        be = AutoToolsBuildEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                be.make()

    def package(self):
        be = AutoToolsBuildEnvironment(self)
        tmp_dir=os.path.join(self.build_folder, "install")
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                be.install(args=["DESTDIR={}".format(tmp_dir)])
        self.copy("*", src=os.path.join(tmp_dir, "usr", "local"))

    def package_info(self):
       self.cpp_info.libs = tools.collect_libs(self)
       
