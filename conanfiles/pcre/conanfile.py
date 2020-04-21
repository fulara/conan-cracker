import os, stat

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class PcreConan(ConanFile):
    settings = "os", "arch"
    name = "pcre"
    #version = "8.44"

    build_requires = (
        "automake/[>=1.16.2]",
        "autoconf/[>=2.69]",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

        return tools.environment_append(at.vars)

    def source(self): 
        tools.get("https://ftp.pcre.org/pub/pcre/pcre-{}.zip".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        
    def build(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    os.chmod("configure", stat.S_IRWXU)
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
       self.cpp_info.libs = tools.collect_libs(self)
