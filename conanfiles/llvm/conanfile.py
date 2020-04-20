import os

from conans import ConanFile, CMake, tools, AutoToolsBuildEnvironment, RunEnvironment


class ClangConan(ConanFile):
    settings = "os", "arch"
    name = "llvm"
    version = "10.0.0"
    generators = "cmake",

    requires = (
        "zlib/1.2.11",
    )

    build_requires = (
        "python/[2.7]"
    )

#   should these options be added?
#    options = {
#        "clang": [True, False],
#        "clange-extra": [True, False],
#        "lldb": [True, False],
#        "compiler-rt": [True, False],
#    }
  
#    build_requires = (
#       ... python on rh6.
#    )
 
    exports_sources = "CMakeLists.txt", "src/*",

    _cmake = None

    @property
    def _source_subfolder(self):
        return "source_subfolder"

    @property
    def _build_subfolder(self):
        return "build_subfolder"

    def _configure_cmake(self):
        if self._cmake:
            return self._cmake
      
        cmake = CMake(self)
        cmake.definitions["LLVM_ENABLE_PROJECTS"] = "clang;clang-tools-extra;lldb;compiler-rt"
        cmake.definitions["LLVM_STATIC_LINK_CXX_STDLIB"] = "ON"
        cmake.definitions["CMAKE_BUILD_TYPE"] = "Release"
        cmake.configure(build_folder = self._build_subfolder, source_folder = os.path.join(self._source_subfolder, "llvm"))

        self._cmake = cmake
        return self._cmake

    def source(self):
        tools.get("https://github.com/llvm/llvm-project/archive/llvmorg-{}.tar.gz".format(self.version))
        os.rename("{}-project-llvmorg-{}".format(self.name, self.version), self._source_subfolder)

   
    def build(self):
        with tools.chdir(self._source_subfolder):
           cmake = self._configure_cmake()
           cmake.build()
   
    def package(self):
        with tools.chdir(self._source_subfolder):
           cmake = self._configure_cmake()
           cmake.install()

    def package_info(self):
        self.cpp_info.libs = tools.collect_libs(self)
