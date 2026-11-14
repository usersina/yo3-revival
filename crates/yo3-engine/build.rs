//! Compiles EDOPro's core and its Lua (fetched by `engine/fetch.sh` into `engine/vendor/ygopro-core`) into this
//! program, the way the core's own build does: C++17 without RTTI, Lua compiled as C++ with the core's configuration
//! header forced in. Without the sources the program is built without a core and says so when started (`no_core`).
use std::path::PathBuf;

/// The core's Lua files (its lua/meson.build).
const LUA: &[&str] = &[
    "lapi", "lauxlib", "lbaselib", "lcode", "lctype", "ldebug", "ldo", "ldump", "lfunc", "lgc", "liolib", "llex",
    "lmathlib", "lmem", "lobject", "lopcodes", "lparser", "lstate", "lstring", "lstrlib", "ltable", "ltablib", "ltm",
    "lundump", "lvm", "lzio",
];
/// The core's own files (its meson.build).
const CORE: &[&str] = &[
    "card", "duel", "effect", "field", "interpreter", "libcard", "libdebug", "libduel", "libeffect", "libgroup",
    "ocgapi", "operations", "playerop", "processor", "processor_visit", "scriptlib",
];

fn main() {
    println!("cargo::rustc-check-cfg=cfg(no_core)");
    println!("cargo::rerun-if-env-changed=YO3_CORE_DIR");
    let dir = std::env::var_os("YO3_CORE_DIR").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../engine/vendor/ygopro-core")
    });
    println!("cargo::rerun-if-changed={}", dir.display());
    let commit = std::process::Command::new("git").arg("-C").arg(&dir).args(["rev-parse", "HEAD"]).output().ok()
        .filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|| "unknown".into());
    println!("cargo::rustc-env=YO3_CORE_COMMIT={commit}");
    if !dir.join("ocgapi.cpp").is_file() || !dir.join("lua/src/lua.h").is_file() {
        println!("cargo::warning=EDOPro's core is missing ({}); run engine/fetch.sh. Building without it.", dir.display());
        println!("cargo::rustc-cfg=no_core");
        return;
    }
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let common = |build: &mut cc::Build| {
        build.cpp(true).std("c++17").flag_if_supported("-fno-rtti").define("NDEBUG", None).warnings(false).opt_level(2);
        // Lua seeds its string hashes from the clock and addresses; a fixed seed keeps a duel's replay exact
        // (`pairs` order over string keys).
        build.define("luai_makeseed(L)", "0x5eed");
        if windows {
            build.define("WIN32", None).define("_WIN32", None).define("NOMINMAX", None);
            // The C++ runtime goes into the program (below), not beside it as libstdc++-6.dll.
            build.cpp_link_stdlib(None);
        }
    };

    let mut core = cc::Build::new();
    common(&mut core);
    core.include(dir.join("lua/src")).include(&dir);
    for f in CORE {
        core.file(dir.join(format!("{f}.cpp")));
    }
    // Our queries over the core's (see the file).
    println!("cargo::rerun-if-changed=src/query.cpp");
    core.file("src/query.cpp");
    core.compile("ocgcore");
    let mut lua = cc::Build::new();
    common(&mut lua);
    lua.flag("-x").flag("c++").flag("-include").flag(dir.join("lua/luaconf-customize.h").to_str().unwrap());
    lua.include(dir.join("lua/src"));
    for f in LUA {
        lua.file(dir.join(format!("lua/src/{f}.c")));
    }
    lua.compile("yo3lua");

    if windows {
        // A self-contained program: no libstdc++/libgcc DLLs beside it.
        println!("cargo::rustc-link-lib=static=stdc++");
        println!("cargo::rustc-link-arg=-static-libgcc");
        println!("cargo::rustc-link-arg=-static");
    }
}
