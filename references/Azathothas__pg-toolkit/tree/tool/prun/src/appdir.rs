//! What a bundle's own directories tell the payload about where things are.
//!
//! ⭐ This is the launcher's real surface. A program does not ask "where is my
//! bundle"; it asks a library, and that library reads a variable naming a
//! directory. So the bundle carries `lib/gtk-4.0/`, and the launcher's job is
//! to know that this means `GTK_PATH`, `GTK_EXE_PREFIX` and an immodules cache
//! somewhere under it.
//!
//! ⛔ Every class here is a capability, TODO/port.md rule 2 binds all of them,
//! and TODO/ledger.json has a row for each. A class dropped because this
//! project has no subject for it today is the exact failure that rule names.
//!
//! ⚠ The lists of directories are read from `lib.path`, whose first component
//! is the class. A bundle with no `lib.path` gets one written when its tree is
//! writable, and is walked in memory when it is not - which is the read-only
//! image case, and it is the common one.
//!
//! SPDX-License-Identifier: 0BSD

use std::fs;
use std::path::{Path, PathBuf};

use crate::envx;
use crate::names;
use crate::report::Report;
use crate::sys;

/// The bundle's layout.
pub struct Bundle {
    pub root: PathBuf,
    pub bin: PathBuf,
    /// The library tree the payload's class selects.
    pub lib: PathBuf,
    pub share: PathBuf,
    pub etc: PathBuf,
}

impl Bundle {
    pub fn new(root: &Path, lib: &Path) -> Bundle {
        Bundle {
            root: root.to_path_buf(),
            bin: root.join("bin"),
            lib: lib.to_path_buf(),
            share: root.join("share"),
            etc: root.join("etc"),
        }
    }
}

/// An external graphics installation the caller pointed at.
pub struct Graphics {
    pub share: PathBuf,
    pub lib: PathBuf,
}

/// Everything the launcher may set, so `--help` and the tests can see the
/// whole surface in one place. ⚠ A variable set by the code and missing here
/// is a capability nobody can find out about, and the test below is what says
/// so.
pub const SURFACE: &[&str] = &[
    "ALSA_CONFIG_PATH",
    "AMDGPU_ASIC_ID_TABLE_PATHS",
    "BABL_PATH",
    "CURL_CA_BUNDLE",
    "DRIRC_CONFIGDIR",
    "FOLKS_BACKEND_PATH",
    "FONTCONFIG_FILE",
    "FREI0R_PATH",
    "GBM_BACKENDS_PATH",
    "GCONV_PATH",
    "GDK_PIXBUF_MODULEDIR",
    "GDK_PIXBUF_MODULE_FILE",
    "GEGL_PATH",
    "GIO_MODULE_DIR",
    "GI_TYPELIB_PATH",
    "GSETTINGS_SCHEMA_DIR",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_SCANNER",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GS_LIB",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GTK_PATH",
    "IMLIB2_FILTER_PATH",
    "IMLIB2_LOADER_PATH",
    "LADSPA_PATH",
    "LIBDECOR_PLUGIN_DIR",
    "LIBGL_DRIVERS_PATH",
    "LIBHEIF_PLUGIN_PATH",
    "LIBTHAI_DICTDIR",
    "LIBVA_DRIVERS_PATH",
    "MAGIC",
    "MAGICK_CODER_FILTER_PATH",
    "MAGICK_CODER_MODULE_PATH",
    "MAGICK_CONFIGURE_PATH",
    "MAGICK_HOME",
    "MLT_PRESETS_PATH",
    "MLT_PROFILES_PATH",
    "MLT_REPOSITORY",
    "OPENSSL_CONF",
    "PATH",
    "PERLLIB",
    "PIPEWIRE_CONFIG_DIR",
    "PIPEWIRE_MODULE_DIR",
    "PYTHONDONTWRITEBYTECODE",
    "PYTHONNOUSERSITE",
    "QT_PLUGIN_PATH",
    "QT_XKB_CONFIG_ROOT",
    "REQUESTS_CA_BUNDLE",
    "SPA_PLUGIN_DIR",
    "SSL_CERT_FILE",
    "TCL_LIBRARY",
    "TERMINFO",
    "TEXTDOMAINDIR",
    "TK_LIBRARY",
    "VK_DRIVER_FILES",
    "WEBKIT_EXEC_PATH",
    "WEBKIT_INJECTED_BUNDLE_PATH",
    "XDG_DATA_DIRS",
    "XKB_CONFIG_ROOT",
    "XLOCALEDIR",
    "XTABLES_LIBDIR",
    "__EGL_VENDOR_LIBRARY_DIRS",
    "__EGL_VENDOR_LIBRARY_FILENAMES",
];

/// Apply everything the bundle's own layout implies.
pub fn apply(b: &Bundle, classes: &[String], gfx: Option<&Graphics>, r: &mut Report) {
    programs(b, r);
    if let Some(g) = gfx {
        graphics_lib(g, r);
    }
    for class in classes {
        library_class(b, class, r);
    }
    share(b, gfx, r);
    etc(b, r);
}

/// What the bundle's `bin/` implies, before any library class.
fn programs(b: &Bundle, r: &mut Report) {
    let launch = b.bin.join("gio-launch-desktop");
    if is_exe(&launch) {
        // ⚠ Not a path list: the variable names one program.
        envx::set("GIO_LAUNCH_DESKTOP", &launch);
        r.say("GIO_LAUNCH_DESKTOP", &launch.to_string_lossy());
    }
    if is_exe(&b.bin.join("python")) || is_exe(&b.bin.join("python3")) {
        // ⛔ Only when the caller has not decided. A user running a bundled
        // interpreter against their own site directory is doing it on purpose.
        if !envx::present("PYTHONNOUSERSITE") {
            envx::set("PYTHONNOUSERSITE", "1");
            r.say("PYTHONNOUSERSITE", "1");
        }
    }
    if let Ok(dir) = b.bin.read_dir() {
        for e in dir.flatten() {
            if e.file_name().to_string_lossy().starts_with("WebKit") && is_exe(&e.path()) {
                envx::set("WEBKIT_EXEC_PATH", &b.bin);
                r.say("WEBKIT_EXEC_PATH", &b.bin.to_string_lossy());
                break;
            }
        }
    }
    envx::prepend("PATH", &b.bin);
}

/// An external graphics installation replaces the bundle's own driver
/// directories. ⚠ It is a caller's instruction rather than a bundle's, and it
/// is the one case where a host directory wins by request.
fn graphics_lib(g: &Graphics, r: &mut Report) {
    let dri = g.lib.join("dri");
    if dri.is_dir() {
        envx::set("LIBGL_DRIVERS_PATH", &dri);
        envx::prepend("LIBVA_DRIVERS_PATH", &dri);
        r.say("LIBGL_DRIVERS_PATH", &dri.to_string_lossy());
    }
    let gbm = g.lib.join("gbm");
    if gbm.is_dir() {
        envx::prepend("GBM_BACKENDS_PATH", &gbm);
        r.say("GBM_BACKENDS_PATH", &gbm.to_string_lossy());
    }
}

/// One directory under the library tree, and what it means.
fn library_class(b: &Bundle, class: &str, r: &mut Report) {
    let dir = b.lib.join(class);
    let d = dir.to_string_lossy().into_owned();
    match class {
        "gconv" => {
            envx::prepend("GCONV_PATH", &dir);
            r.say("GCONV_PATH", &d);
        }
        "gio" => {
            let modules = dir.join("modules");
            if modules.exists() {
                envx::set("GIO_MODULE_DIR", &modules);
                r.say("GIO_MODULE_DIR", &modules.to_string_lossy());
            }
        }
        "dri" => {
            envx::set("LIBGL_DRIVERS_PATH", &dir);
            nvidia_first_va(r);
            envx::prepend("LIBVA_DRIVERS_PATH", &dir);
            r.say("LIBGL_DRIVERS_PATH", &d);
        }
        "gbm" => {
            for host in host_gbm() {
                envx::append("GBM_BACKENDS_PATH", host);
            }
            envx::prepend("GBM_BACKENDS_PATH", &dir);
            r.say("GBM_BACKENDS_PATH", &envx::get("GBM_BACKENDS_PATH"));
        }
        "libheif" => {
            let plugins = dir.join("plugins");
            let at = if plugins.exists() { plugins } else { dir.clone() };
            envx::set("LIBHEIF_PLUGIN_PATH", &at);
            r.say("LIBHEIF_PLUGIN_PATH", &at.to_string_lossy());
        }
        "xtables" => {
            envx::set("XTABLES_LIBDIR", &dir);
            r.say("XTABLES_LIBDIR", &d);
        }
        "folks" => {
            if let Some(p) = find(&dir, 4, |name, ft| ft.is_dir() && name == "backends") {
                envx::set("FOLKS_BACKEND_PATH", &p);
                r.say("FOLKS_BACKEND_PATH", &p.to_string_lossy());
            }
        }
        "imlib2" => {
            for (sub, var) in [("loaders", "IMLIB2_LOADER_PATH"), ("filters", "IMLIB2_FILTER_PATH")] {
                let p = dir.join(sub);
                if p.exists() {
                    envx::set(var, &p);
                    r.say(var, &p.to_string_lossy());
                }
            }
        }
        "libdecor" => {
            let plugins = dir.join("plugins-1");
            if plugins.exists() {
                envx::set("LIBDECOR_PLUGIN_DIR", &plugins);
                r.say("LIBDECOR_PLUGIN_DIR", &plugins.to_string_lossy());
            }
        }
        "ladspa" => {
            envx::set("LADSPA_PATH", &dir);
            r.say("LADSPA_PATH", &d);
        }
        _ => library_class_prefix(b, class, &dir, r),
    }
}

/// The versioned classes: a directory whose name carries a version, so the
/// match is on a prefix rather than on the whole name.
fn library_class_prefix(b: &Bundle, class: &str, dir: &Path, r: &mut Report) {
    let d = dir.to_string_lossy().into_owned();
    if class.starts_with("python") {
        // ⛔ Only when the bundle cannot be written to. A writable bundle
        // caching bytecode beside its own modules is the fast path; a
        // read-only one would try, fail, and print a warning per module.
        if !sys::writable(&b.root) {
            envx::set("PYTHONDONTWRITEBYTECODE", "1");
            r.say("PYTHONDONTWRITEBYTECODE", "1");
        }
        return;
    }
    if class.starts_with("perl") {
        perl(dir, r);
        return;
    }
    if class.starts_with("girepository-") {
        envx::set("GI_TYPELIB_PATH", dir);
        r.say("GI_TYPELIB_PATH", &d);
        return;
    }
    if class.starts_with("spa-") {
        envx::set("SPA_PLUGIN_DIR", dir);
        r.say("SPA_PLUGIN_DIR", &d);
        return;
    }
    if class.starts_with("pipewire-") {
        envx::set("PIPEWIRE_MODULE_DIR", dir);
        r.say("PIPEWIRE_MODULE_DIR", &d);
        return;
    }
    if class.starts_with("webkit") {
        if let Some(p) = find(dir, 6, |name, ft| {
            ft.is_file() && name.starts_with("libwebkit") && name.ends_with("gtkinjectedbundle.so")
        }) {
            if let Some(parent) = p.parent() {
                envx::set("WEBKIT_INJECTED_BUNDLE_PATH", parent);
                r.say("WEBKIT_INJECTED_BUNDLE_PATH", &parent.to_string_lossy());
            }
        }
        return;
    }
    if class.starts_with("gtk-") {
        envx::prepend("GTK_PATH", dir);
        envx::set("GTK_EXE_PREFIX", &b.root);
        envx::set("GTK_DATA_PREFIX", &b.root);
        r.say("GTK_PATH", &envx::get("GTK_PATH"));
        if let Some(p) = find(dir, 6, |name, ft| ft.is_file() && name == "immodules.cache") {
            envx::set("GTK_IM_MODULE_FILE", &p);
            r.say("GTK_IM_MODULE_FILE", &p.to_string_lossy());
        }
        return;
    }
    if class.starts_with("qt") {
        // ⚠ A `qt.conf` beside the payload is Qt's own answer and it wins. Two
        // mechanisms saying different things is how a plugin gets loaded
        // twice from two prefixes.
        let plugins = dir.join("plugins");
        if plugins.exists() && !b.bin.join("qt.conf").exists() {
            envx::prepend("QT_PLUGIN_PATH", &plugins);
            r.say("QT_PLUGIN_PATH", &envx::get("QT_PLUGIN_PATH"));
        }
        return;
    }
    if class.starts_with("babl-") {
        envx::set("BABL_PATH", dir);
        r.say("BABL_PATH", &d);
        return;
    }
    if class.starts_with("gegl-") {
        envx::set("GEGL_PATH", dir);
        r.say("GEGL_PATH", &d);
        return;
    }
    if class.starts_with("frei0r-") {
        envx::set("FREI0R_PATH", dir);
        r.say("FREI0R_PATH", &d);
        return;
    }
    if class.starts_with("mlt-") {
        envx::set("MLT_REPOSITORY", dir);
        r.say("MLT_REPOSITORY", &d);
        return;
    }
    if class.starts_with("ImageMagick-") {
        image_magick(b, dir, r);
        return;
    }
    if class.starts_with("tcl") {
        if dir.join("msgs").exists() {
            envx::prepend("TCL_LIBRARY", dir);
            r.say("TCL_LIBRARY", &d);
            let tk = b.lib.join(class.replacen("tcl", "tk", 1));
            if tk.exists() {
                envx::prepend("TK_LIBRARY", &tk);
                r.say("TK_LIBRARY", &tk.to_string_lossy());
            }
        }
        return;
    }
    if class.starts_with("gstreamer-") {
        for var in ["GST_PLUGIN_PATH", "GST_PLUGIN_SYSTEM_PATH", "GST_PLUGIN_SYSTEM_PATH_1_0"] {
            envx::prepend(var, dir);
        }
        r.say("GST_PLUGIN_PATH", &envx::get("GST_PLUGIN_PATH"));
        let scanner = dir.join("gst-plugin-scanner");
        if scanner.exists() {
            envx::set("GST_PLUGIN_SCANNER", &scanner);
            r.say("GST_PLUGIN_SCANNER", &scanner.to_string_lossy());
        }
        return;
    }
    if class.starts_with("gdk-pixbuf-") {
        let mut set_dir = false;
        let mut set_cache = false;
        walk(dir, 6, &mut |p: &Path, name: &str, ft: &fs::FileType| {
            if name == "loaders" && ft.is_dir() && !set_dir {
                envx::set("GDK_PIXBUF_MODULEDIR", p);
                set_dir = true;
            }
            if name == "loaders.cache" && ft.is_file() && !set_cache {
                envx::set("GDK_PIXBUF_MODULE_FILE", p);
                set_cache = true;
            }
            set_dir && set_cache
        });
        if set_dir {
            r.say("GDK_PIXBUF_MODULEDIR", &envx::get("GDK_PIXBUF_MODULEDIR"));
        }
        if set_cache {
            r.say("GDK_PIXBUF_MODULE_FILE", &envx::get("GDK_PIXBUF_MODULE_FILE"));
        }
    }
}

/// Perl's module tree, and a defect not carried across.
///
/// ⛔ The launcher this replaces sets `PERLLIB` to the versioned directory
/// itself. On more than one distribution the modules are one level further
/// down, in `core_perl`, `site_perl` and `vendor_perl`, and the variable then
/// names a directory with no modules in it. Upstream's own tracker carries the
/// report, still open, with a bundle carrying a hand-written workaround.
///
/// ⭐ TODO/port.md rule 4: reproducing a defect because upstream has it is a
/// failed port. The parent directory is kept - a layout that does not split
/// still works - and each split directory found is added as well.
fn perl(dir: &Path, r: &mut Report) {
    envx::prepend("PERLLIB", dir);
    let mut added = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((at, depth)) = stack.pop() {
        if depth > 2 {
            continue;
        }
        let Ok(entries) = at.read_dir() else { continue };
        for e in entries.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), "core_perl" | "site_perl" | "vendor_perl") {
                envx::prepend("PERLLIB", e.path());
                added.push(name);
            } else {
                stack.push((e.path(), depth + 1));
            }
        }
    }
    r.say("PERLLIB", &envx::get("PERLLIB"));
    if !added.is_empty() {
        r.note(&format!("perl: added {} split module directories", added.len()));
    }
}

fn image_magick(b: &Bundle, dir: &Path, r: &mut Report) {
    envx::set("MAGICK_HOME", &b.root);
    for (want, var) in [("coders", "MAGICK_CODER_MODULE_PATH"), ("filters", "MAGICK_CODER_FILTER_PATH")] {
        if let Some(p) = find(dir, 2, |name, ft| ft.is_dir() && name == want) {
            envx::set(var, &p);
            r.say(var, &p.to_string_lossy());
        }
    }
    if let Ok(entries) = b.etc.read_dir() {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with("ImageMagick-") && e.path().is_dir() {
                envx::set("MAGICK_CONFIGURE_PATH", e.path());
                r.say("MAGICK_CONFIGURE_PATH", &e.path().to_string_lossy());
                break;
            }
        }
    }
}

/// The host's video-acceleration directories, ahead of the bundle's, when the
/// proprietary driver is loaded.
///
/// ⭐ It is conditional on the driver being present rather than on a
/// preference: with that driver in the kernel, the acceleration back end has
/// to be the host's, because it is the counterpart of the module.
fn nvidia_first_va(r: &mut Report) {
    if names::peek("PRUN_NO_NVIDIA_PRIME") == "1" {
        return;
    }
    if !Path::new("/sys/module/nvidia/version").exists() {
        return;
    }
    for d in host_va() {
        envx::append("LIBVA_DRIVERS_PATH", d);
    }
    r.note("nvidia: host video-acceleration directories admitted");
}

/// ⚠ The order is the artefact's own, and it is not the order its source
/// reads in. Upstream names these four low-to-high and then adds each with a
/// call that PREPENDS, so the list it ends up handing the driver is the
/// reverse of the one written down. Parity is the floor, so what is written
/// here is the order that actually reaches the payload.
fn host_va() -> Vec<&'static str> {
    let mut v = vec![triple_sub("dri")];
    v.extend(["/usr/lib64/dri", "/usr/lib/dri", "/run/opengl-driver/lib/dri"]);
    v.retain(|d| !d.is_empty());
    v
}

fn host_gbm() -> Vec<&'static str> {
    let mut v = vec![triple_sub("gbm")];
    v.extend(["/usr/lib64/gbm", "/usr/lib/gbm", "/run/opengl-driver/lib/gbm"]);
    v.retain(|d| !d.is_empty());
    v
}

/// The multiarch subdirectory for this architecture.
///
/// ⚠ An architecture with no multiarch convention answers with the empty
/// string, which the caller drops. An absence is not a zero and this one is
/// reported by the launcher when it reports its search path.
fn triple_sub(kind: &str) -> &'static str {
    macro_rules! pick {
        ($dri:expr, $gbm:expr) => {
            if kind == "dri" {
                $dri
            } else {
                $gbm
            }
        };
    }
    #[cfg(target_arch = "x86_64")]
    {
        pick!("/usr/lib/x86_64-linux-gnu/dri", "/usr/lib/x86_64-linux-gnu/gbm")
    }
    #[cfg(target_arch = "aarch64")]
    {
        pick!("/usr/lib/aarch64-linux-gnu/dri", "/usr/lib/aarch64-linux-gnu/gbm")
    }
    #[cfg(target_arch = "riscv64")]
    {
        pick!("/usr/lib/riscv64-linux-gnu/dri", "/usr/lib/riscv64-linux-gnu/gbm")
    }
    #[cfg(target_arch = "loongarch64")]
    {
        pick!("/usr/lib/loongarch64-linux-gnu/dri", "/usr/lib/loongarch64-linux-gnu/gbm")
    }
    #[cfg(all(target_arch = "powerpc64", target_endian = "big"))]
    {
        pick!("/usr/lib/powerpc64-linux-gnu/dri", "/usr/lib/powerpc64-linux-gnu/gbm")
    }
    #[cfg(all(target_arch = "powerpc64", target_endian = "little"))]
    {
        pick!("/usr/lib/powerpc64le-linux-gnu/dri", "/usr/lib/powerpc64le-linux-gnu/gbm")
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64",
        target_arch = "powerpc64"
    )))]
    {
        let _ = kind;
        ""
    }
}

/// The data directories, and everything a class under `share/` implies.
fn share(b: &Bundle, gfx: Option<&Graphics>, r: &mut Report) {
    let gfx_share = gfx.map(|g| g.share.clone());
    let have_share = b.share.exists();
    if !have_share && gfx_share.is_none() {
        return;
    }

    // ⛔ Order is priority, lowest first, because each call puts its argument
    // at the FRONT. The bundle's own share wins, then the graphics
    // installation, then the user's, then the system's.
    for d in [
        "/etc",
        "/run/current-system/sw/share",
        "/run/opengl-driver/share",
        "/usr/share",
        "/usr/local/share",
    ] {
        envx::prepend("XDG_DATA_DIRS", d);
    }
    let home = envx::get("HOME");
    if !home.is_empty() {
        envx::prepend("XDG_DATA_DIRS", format!("{home}/.local/share"));
    }
    if have_share {
        envx::prepend("XDG_DATA_DIRS", &b.share);
    }
    if let Some(g) = &gfx_share {
        envx::prepend("XDG_DATA_DIRS", g);
    }
    r.say("XDG_DATA_DIRS", &envx::get("XDG_DATA_DIRS"));

    if have_share {
        share_classes(&b.share, r);
        // ⛔ ONE place decides this, and it is not inside `share_classes`.
        // That function matches on a share subdirectory's NAME, and the keymap
        // data is not always under one it knows - so a second arm that could
        // also set the variable would set it from whichever directory
        // `read_dir` happened to hand back first.
        if let Some(xkb) = xkb_root(&b.share) {
            if !Path::new("/usr/share/X11/xkb").exists() {
                envx::set("XKB_CONFIG_ROOT", &xkb);
                envx::set("QT_XKB_CONFIG_ROOT", &xkb);
                r.say("XKB_CONFIG_ROOT", &xkb.to_string_lossy());
            }
        }
    }
    // The graphics classes come from the external installation when there is
    // one, and from the bundle's own share otherwise.
    let gdir = gfx_share.unwrap_or_else(|| b.share.clone());
    if gdir.exists() {
        graphics_share(&gdir, r);
    }
}

/// xkb_root is the directory xkbcommon should read keymap data from, or None
/// when the bundle carries none.
///
/// ⛔ **A GTK bundle does not start under WAYLAND without this.** xkbcommon's
/// compiled-in default names the store path it was built against, and
/// xkeyboard-config 2.48 does not put its data where that default points - not
/// in the bundle and not in the closure either, where it is at
/// `share/xkeyboard-config-N`. The payload dies before it maps anything:
///
/// ```text
/// xkbcommon: ERROR: Couldn't find file "rules/evdev" in include paths
/// Gdk-ERROR **: Failed to create XKB keymap
/// ```
///
/// ⚠ This looked only at `X11/xkb` until T-106's Wayland reading, which could
/// not be taken until it was fixed. The X11 backend never needed it - the
/// server owns the keymap there - so nothing had asked the question.
///
/// ⛔ The ORDER is a rule rather than a preference: `X11/xkb` is where
/// xkbcommon looks and where every distribution puts it, so a bundle carrying
/// both uses the conventional one. ⚠ `rules/` has to be there. A directory
/// without it is not keymap data, and pointing the variable at one turns a
/// failure xkbcommon explains into one it does not.
fn xkb_root(share: &Path) -> Option<PathBuf> {
    let conventional = share.join("X11").join("xkb");
    if conventional.join("rules").is_dir() {
        return Some(conventional);
    }
    let entries = share.read_dir().ok()?;
    for e in entries.flatten() {
        if !e.file_name().to_string_lossy().starts_with("xkeyboard-config-") {
            continue;
        }
        let p = e.path();
        if p.join("rules").is_dir() {
            return Some(p);
        }
    }
    None
}

fn share_classes(share: &Path, r: &mut Report) {
    let Ok(entries) = share.read_dir() else { return };
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        match name.as_str() {
            "alsa" => {
                // ⚠ Only when the host has none. A host configuration that
                // exists is the one the host's sound daemon agrees with.
                let conf = p.join("alsa.conf");
                if !Path::new("/usr/share/alsa/alsa.conf").exists() && conf.exists() {
                    envx::set("ALSA_CONFIG_PATH", &conf);
                    r.say("ALSA_CONFIG_PATH", &conf.to_string_lossy());
                }
            }
            "X11" => {
                // ⚠ The keymap data is decided by `xkb_root` and not here: it
                // is not always under X11/, and read_dir hands these back in no
                // order, so a second place that could also set the variable
                // would set it from whichever directory came first.
                let loc = p.join("locale");
                if !Path::new("/usr/share/X11/locale").exists() && loc.exists() {
                    envx::set("XLOCALEDIR", &loc);
                    r.say("XLOCALEDIR", &loc.to_string_lossy());
                }
            }
            "libthai" => {
                if p.join("thbrk.tri").exists() {
                    envx::set("LIBTHAI_DICTDIR", &p);
                    r.say("LIBTHAI_DICTDIR", &p.to_string_lossy());
                }
            }
            "glib-2.0" => {
                add_from_data_dirs("GSETTINGS_SCHEMA_DIR", "glib-2.0/schemas", r);
            }
            "terminfo" => {
                envx::set("TERMINFO", &p);
                r.say("TERMINFO", &p.to_string_lossy());
            }
            "locale" => {
                envx::set("TEXTDOMAINDIR", &p);
                r.say("TEXTDOMAINDIR", &p.to_string_lossy());
            }
            "file" => {
                let magic = p.join("misc/magic.mgc");
                if magic.exists() {
                    envx::set("MAGIC", &magic);
                    r.say("MAGIC", &magic.to_string_lossy());
                }
            }
            "ghostscript" => {
                if let Some(base) = ghostscript_base(&p) {
                    let v = format!(
                        "{}:{}",
                        base.join("Init").to_string_lossy(),
                        base.to_string_lossy()
                    );
                    envx::set("GS_LIB", &v);
                    r.say("GS_LIB", &v);
                }
            }
            "pipewire" => {
                if !Path::new("/usr/share/pipewire").exists()
                    && !envx::present("PIPEWIRE_CONFIG_DIR")
                {
                    envx::set("PIPEWIRE_CONFIG_DIR", &p);
                    r.say("PIPEWIRE_CONFIG_DIR", &p.to_string_lossy());
                }
            }
            other if other.starts_with("mlt-") => {
                for (sub, var) in [("profiles", "MLT_PROFILES_PATH"), ("presets", "MLT_PRESETS_PATH")] {
                    let q = p.join(sub);
                    if q.exists() {
                        envx::set(var, &q);
                        r.say(var, &q.to_string_lossy());
                    }
                }
            }
            _ => {}
        }
    }
}

fn ghostscript_base(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = dir.read_dir() {
        for e in entries.flatten() {
            if e.path().join("Resource").join("Init").is_dir() {
                return Some(e.path().join("Resource"));
            }
        }
    }
    if dir.join("Resource").join("Init").is_dir() {
        return Some(dir.join("Resource"));
    }
    None
}

/// The graphics vendor lists, which are the fiddliest part of the surface.
fn graphics_share(gdir: &Path, r: &mut Report) {
    let Ok(entries) = gdir.read_dir() else { return };
    let prime = names::peek("PRUN_NO_NVIDIA_PRIME") != "1"
        && Path::new("/sys/module/nvidia/version").exists();
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        match e.file_name().to_string_lossy().as_ref() {
            "glvnd" => {
                // ⛔ Empty counts as absent here, and only here. libglvnd
                // reads this variable and RETURNS, so a host exporting it
                // empty leaves the bundle with no vendor at all; overwriting
                // an empty one is the difference between EGL working and the
                // payload drawing nothing.
                if prime && envx::get("__EGL_VENDOR_LIBRARY_FILENAMES").is_empty() {
                    if let Some(list) = vendor_files_nvidia_first() {
                        envx::set("__EGL_VENDOR_LIBRARY_FILENAMES", &list);
                        r.say("__EGL_VENDOR_LIBRARY_FILENAMES", &list);
                    }
                }
                add_from_data_dirs("__EGL_VENDOR_LIBRARY_DIRS", "glvnd/egl_vendor.d", r);
            }
            "vulkan" => vulkan(gdir, r),
            "drirc.d" => {
                if !Path::new("/usr/share/drirc.d").exists() {
                    envx::set("DRIRC_CONFIGDIR", &p);
                    r.say("DRIRC_CONFIGDIR", &p.to_string_lossy());
                }
            }
            "libdrm" => {
                // ⚠ The HOST's tables first and the bundle's last, which is
                // the order the artefact this replaces ends up with and the
                // opposite of the order its source reads in. It is kept
                // because it is also the right one: the file describes the
                // GPUs the host's kernel knows about, so a host that has one
                // knows about newer hardware than any bundle could carry, and
                // a host that has none falls through to the bundle's.
                for host in ["/usr/local/share/libdrm", "/usr/share/libdrm"] {
                    envx::append("AMDGPU_ASIC_ID_TABLE_PATHS", host);
                }
                envx::append("AMDGPU_ASIC_ID_TABLE_PATHS", &p);
                r.say("AMDGPU_ASIC_ID_TABLE_PATHS", &envx::get("AMDGPU_ASIC_ID_TABLE_PATHS"));
            }
            _ => {}
        }
    }
}

/// The Vulkan driver files.
///
/// ⭐ The bundle's own driver files, and the HOST's proprietary ones - never
/// the host's generic ones unless the caller asked. A host Vulkan driver built
/// against a newer libc than the bundle carries is the case this ordering
/// exists for, and the proprietary one is the exception because it is the
/// counterpart of a kernel module.
fn vulkan(gdir: &Path, r: &mut Report) {
    const VAR: &str = "VK_DRIVER_FILES";
    let all_host = names::peek("PRUN_HOST_VULKAN_ICD") == "1";
    let dirs = envx::get("XDG_DATA_DIRS");
    for data in dirs.split(':').rev() {
        if !envx::usable(data) {
            continue;
        }
        let icd = Path::new(data).join("vulkan/icd.d");
        if !icd.exists() {
            continue;
        }
        if data.starts_with(&*gdir.to_string_lossy()) || all_host {
            envx::prepend(VAR, &icd);
            continue;
        }
        let Ok(entries) = icd.read_dir() else { continue };
        for e in entries.flatten() {
            if e.path().is_file() && e.file_name().to_string_lossy().contains("nvidia") {
                envx::prepend(VAR, e.path());
            }
        }
    }
    if envx::present(VAR) {
        r.say(VAR, &envx::get(VAR));
    }
}

/// The EGL vendor list, with the proprietary vendor first.
///
/// ⛔ `__EGL_VENDOR_LIBRARY_FILENAMES` REPLACES the directory list rather than
/// adding to it: libglvnd reads it and returns, so a host that exports it
/// makes a bundle setting only the directory variable load the HOST's vendor
/// files. Setting it empty is worse than not setting it, because the empty
/// string is still a value and the branch is still taken.
fn vendor_files_nvidia_first() -> Option<String> {
    let dirs = envx::get("XDG_DATA_DIRS");
    let mut nvidia: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    for data in dirs.split(':') {
        if !envx::usable(data) {
            continue;
        }
        let Ok(entries) = Path::new(data).join("glvnd/egl_vendor.d").read_dir() else { continue };
        let mut here: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
            .collect();
        here.sort();
        for p in here {
            let s = p.to_string_lossy().into_owned();
            if p.file_name().unwrap_or_default().to_string_lossy().contains("nvidia") {
                nvidia.push(s);
            } else {
                rest.push(s);
            }
        }
    }
    if nvidia.is_empty() {
        return None;
    }
    nvidia.extend(rest);
    Some(nvidia.join(":"))
}

/// Add one relative directory from every data directory that has it.
fn add_from_data_dirs(var: &str, rel: &str, r: &mut Report) {
    let dirs = envx::get("XDG_DATA_DIRS");
    for data in dirs.split(':').rev() {
        if !envx::usable(data) {
            continue;
        }
        let p = Path::new(data).join(rel);
        if p.exists() {
            envx::prepend(var, &p);
        }
    }
    if envx::present(var) {
        r.say(var, &envx::get(var));
    }
}

/// What the bundle's `etc/` implies, and the certificate fallback.
fn etc(b: &Bundle, r: &mut Report) {
    if let Ok(entries) = b.etc.read_dir() {
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            match e.file_name().to_string_lossy().as_ref() {
                "fonts" => {
                    let conf = p.join("fonts.conf");
                    if !Path::new("/etc/fonts/fonts.conf").exists() && conf.exists() {
                        envx::set("FONTCONFIG_FILE", &conf);
                        r.say("FONTCONFIG_FILE", &conf.to_string_lossy());
                    }
                }
                "ssl" => {
                    let conf = p.join("openssl.cnf");
                    if conf.exists() {
                        envx::set("OPENSSL_CONF", &conf);
                        r.say("OPENSSL_CONF", &conf.to_string_lossy());
                    }
                }
                _ => {}
            }
        }
    }
    certificates(r);
}

/// Where the host keeps its certificate authorities, when it does not keep
/// them where a bundled library was built to look.
///
/// ⚠ Nine distributions, four layouts. The bundle carries a library built
/// against ONE of them, so a host using another has certificates the payload
/// cannot find and every fetch fails with a verification error that reads like
/// a network problem.
const CERT_FILES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/cert.pem",
    "/etc/pki/tls/cacert.pem",
    "/etc/ssl/cert.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    "/var/lib/ca-certificates/ca-bundle.pem",
    "/etc/ssl/ca-bundle.pem",
];

fn certificates(r: &mut Report) {
    if Path::new(CERT_FILES[0]).exists() {
        return;
    }
    let Some(found) = CERT_FILES.iter().find(|p| Path::new(p).exists()) else {
        r.note("no certificate bundle found on this host; a payload that verifies will fail");
        return;
    };
    for var in ["REQUESTS_CA_BUNDLE", "CURL_CA_BUNDLE", "SSL_CERT_FILE"] {
        if !envx::present(var) {
            envx::set(var, found);
        }
    }
    r.say("SSL_CERT_FILE", found);
}

fn is_exe(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(p) {
        Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

/// Find the first entry under a directory matching a predicate.
fn find<F>(root: &Path, depth: usize, pred: F) -> Option<PathBuf>
where
    F: Fn(&str, &fs::FileType) -> bool,
{
    let mut hit = None;
    walk(root, depth, &mut |p: &Path, name: &str, ft: &fs::FileType| {
        if pred(name, ft) {
            hit = Some(p.to_path_buf());
            return true;
        }
        false
    });
    hit
}

/// A bounded, breadth-first walk. ⛔ Bounded because a bundle's plugin tree can
/// contain a symlink loop, and because every one of these lookups happens on
/// the path to starting a program: an unbounded walk of a large `share/` is
/// start-up time a user waits for.
fn walk<F>(root: &Path, depth: usize, f: &mut F)
where
    F: FnMut(&Path, &str, &fs::FileType) -> bool,
{
    let mut level = vec![root.to_path_buf()];
    for _ in 0..depth {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(entries) = dir.read_dir() else { continue };
            for e in entries.flatten() {
                let Ok(ft) = e.file_type() else { continue };
                let name = e.file_name().to_string_lossy().into_owned();
                if f(&e.path(), &name, &ft) {
                    return;
                }
                if ft.is_dir() {
                    next.push(e.path());
                }
            }
        }
        if next.is_empty() {
            return;
        }
        level = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_is_sorted_and_unique() {
        let mut s = SURFACE.to_vec();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), SURFACE.len(), "SURFACE has a repeat");
        assert_eq!(s, SURFACE.to_vec(), "SURFACE is not sorted");
    }

    #[test]
    fn the_certificate_list_starts_with_the_one_it_tests_for() {
        assert_eq!(CERT_FILES[0], "/etc/ssl/certs/ca-certificates.crt");
        assert!(CERT_FILES.len() > 1);
    }

    // ⛔ The order is a rule, and the `rules/` requirement is the one that stops
    // this variable pointing somewhere xkbcommon then fails INSIDE - which turns
    // an error it explains into one it does not.
    #[test]
    fn the_keymap_root_prefers_the_conventional_path() {
        // ⚠ The pid is in the name. A fixed path under /tmp is refused for a
        // second uid by fs.protected_regular, and reads like a broken checkout.
        let base = std::env::temp_dir().join(format!("prun-xkb-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        assert!(xkb_root(&base).is_none(), "no data at all is None, not a guess");

        // A directory with no rules/ in it is not keymap data.
        fs::create_dir_all(base.join("xkeyboard-config-9")).unwrap();
        assert!(xkb_root(&base).is_none(), "a directory without rules/ is not data");

        fs::create_dir_all(base.join("xkeyboard-config-2/rules")).unwrap();
        assert_eq!(xkb_root(&base), Some(base.join("xkeyboard-config-2")));

        fs::create_dir_all(base.join("X11/xkb/rules")).unwrap();
        assert_eq!(
            xkb_root(&base),
            Some(base.join("X11").join("xkb")),
            "the conventional path wins over the versioned one"
        );
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn the_walk_stops_at_its_depth() {
    // ⚠ The pid is in the name, and it is not decoration. `fs.protected_regular`
    // is 1 on an ordinary kernel, which refuses an O_CREAT open of an existing
    // file in a sticky world-writable directory when the opener does not own it.
    // A fixed name under /tmp therefore works until somebody runs this suite as a
    // second uid, and then fails as PermissionDenied for a reason that reads like
    // a broken checkout. Measured: five of these failed exactly that way after an
    // earlier run had left the files behind owned by another account.
        let base = std::env::temp_dir().join(format!("prun-walk-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("a/b/c/d")).unwrap();
        fs::write(base.join("a/b/c/d/deep"), b"x").unwrap();
        // `deep` is five components below the root, so four levels of walk
        // must not reach it and five must.
        assert!(find(&base, 4, |n, _| n == "deep").is_none());
        assert!(find(&base, 5, |n, _| n == "deep").is_some());
        fs::remove_dir_all(&base).unwrap();
    }
}
