use std::path::Path;

pub trait QuestCpp: Sized {
    fn add_il2cpp_includes(&mut self, include_dir: &Path) -> &mut Self;
    fn add_fmt_includes(&mut self, include_dir: &Path) -> &mut Self;
    fn add_cordl_includes(&mut self, include_dir: &Path) -> &mut Self;

    fn add_quest_defines(&mut self) -> &mut Self;
    fn add_quest_defaults(&mut self) -> &mut Self;
}

impl QuestCpp for cc::Build {
    /// Add il2cpp includes
    fn add_il2cpp_includes(&mut self, include_dir: &Path) -> &mut Self {
        self.include(
            include_dir
                .join("libil2cpp")
                .join("il2cpp")
                .join("libil2cpp"),
        )
        .include(
            include_dir
                .join("libil2cpp")
                .join("il2cpp")
                .join("external")
                .join("baselib")
                .join("Include"),
        )
        .include(
            include_dir
                .join("libil2cpp")
                .join("il2cpp")
                .join("external")
                .join("baselib")
                .join("Platforms")
                .join("Android")
                .join("Include"),
        )
    }

    /// Add fmt includes
    fn add_fmt_includes(&mut self, include_dir: &Path) -> &mut Self {
        self.include(include_dir.join("fmt").join("fmt").join("include"))
    }

    /// Add bs-cordl includes
    fn add_cordl_includes(&mut self, include_dir: &Path) -> &mut Self {
        self.include(include_dir.join("bs-cordl").join("include"))
    }

    /// Add quest useful defines
    // TODO: Unity defines belong here?
    fn add_quest_defines(&mut self) -> &mut Self {
        self.define("QUEST", None)
            .define("UNITY_2021", None)
            .define("UNITY_2022", None)
            .define("HAS_CODEGEN", None)
            .define("NEED_UNSAFE_CSHARP", None)
            .define("FMT_HEADER_ONLY", None)
    }

    /// Add quest useful default flags
    fn add_quest_defaults(&mut self) -> &mut Self {
        self.pic(true)
            .cpp(true)
            .flag_if_supported("-std=gnu++20")
            .flag_if_supported("-fPIC")
            .flag_if_supported("-fPIE")
            .flag_if_supported("-frtti")
            .flag_if_supported("-fexceptions")
            .flag_if_supported("-fdeclspec")
            .flag_if_supported("-Wno-invalid-offsetof")
            .cpp_link_stdlib("c++_static") // use libstdc++
    }
}
