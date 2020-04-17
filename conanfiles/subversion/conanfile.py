import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "subversion"
    version = "1.13.0"

    requires = (
        "sqlite3/3.31.1",
        "apr/1.7.0",
        "apr-util/1.6.1",
        "zlib/1.2.11", 
        "lz4/1.9.2", 
        "utf8proc/2.5.0",
        "swig/4.0.1",
        "boost/1.72.0",
    )

    build_requires = (
        "autoconf/2.69",
        "libtool/2.4.6",
        "gnu-m4/1.4.18",
    )
    
    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"
   
    def source(self): 
        tools.get("https://github.com/apache/subversion/archive/{}.tar.gz".format(self.version))
        os.rename("{}-{}".format(self.name, self.version), self._source_subfolder)
        
    def build(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    self.run("./autogen.sh")
                    be.configure()
                    be.make()
                    be.make(args=["swig-pl"])
                    be.make(args=["check-swig-pl"])


    def package(self):
        be = AutoToolsBuildEnvironment(self)
        re = RunEnvironment(self)
        with tools.chdir(self._source_subfolder):
            with tools.environment_append(be.vars):
                with tools.environment_append(re.vars):
                    be.install()
                    os.environ['LD_LIBRARY_PATH'] += "{}{}".format(os.pathsep, os.path.join(self.package_folder, "lib"))
                    be.make(args=["install-swig-pl"])
        self.run("rm {}{}{}".format(os.path.join(self.package_folder, "lib"), os.path.sep, "*swig*perl*.so*"))

    def package_info(self):

       self.cpp_info.libs = tools.collect_libs(self)
       self.env_info.PERL5LIB.append(os.path.join(self.package_folder, "lib64", "perl5"))
