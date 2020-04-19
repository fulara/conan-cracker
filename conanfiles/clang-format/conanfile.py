import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment


class GitConan(ConanFile):
    settings = "os", "arch"
    name = "clang-format"

    def build_requirements(self):
        self.build_requires("llvm/{}".format(self.version))
    
    def package(self):
        llvm_path = self.deps_cpp_info["llvm"].rootpath
        self.copy("clang-format", src=os.path.join(llvm_path, "bin"), dst="bin")
