import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class AutomakeConan(ConanFile):
    settings = "os", "arch"
    name = "automake"
    #version = "1.16.2"

    build_requires = (
        "m4/[>=1.4.18]",
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
        tools.get("https://ftp.gnu.org/gnu/automake/automake-{}.tar.xz".format(self.version))
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

    def package_info(self):
        ver = self.version.minor(False).replace(".", "-")
        self.env_info.AUTOMAKE_LIBDIR = os.path.join(self.package_folder, "share", "automake-{}".format(ver))
        self.env_info.PERL5LIB.append(os.path.join(self.package_folder, "share", "automake-{}".format(ver)))
