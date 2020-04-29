import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "autoconf"
    #version = "2.69"

    build_requires = (
<<<<<<< HEAD
        "m4/[>=1.4.18]",
=======
        "m4/1.4.18",
>>>>>>> cracker updates.
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
                                                              
    def package_info(self):
        self.env_info.AUTOCONF = os.path.join(self.package_folder, "bin", "autoconf")
        self.env_info.AUTOM4TE = os.path.join(self.package_folder, "bin", "autom4te")
        autoconf_dir = os.path.join(self.package_folder, "share", "autoconf")
        self.env_info.AC_MACRODIR = os.path.join(self.package_folder, "share", "autoconf")
        cfg = os.path.join(self.package_folder, "share", "autoconf", "autom4te.cfg")
        self.env_info.AUTOM4TE_CFG = os.path.join(self.package_folder, "share", "autoconf", "autom4te.cfg")
        
        self.env_info.autom4te_perllibdir.append(os.path.join(self.package_folder, "share", "autoconf"))
        self.env_info.PERL5LIB.append(os.path.join(self.package_folder, "share", "autoconf"))

        #uber hacky but maybe works.
        # unfortunatelly autom4te.cfg has hardcoded strings.. so we just rewrite them everytime!
        self.run("sed -i -r \"s@prepend-include .*@prepend-include '{}'@g\" {}".format(autoconf_dir, cfg))
        
