//! The test host of the native-static rung:
//! `podvm-dispatch-run ROOT GUEST-PATH ARGV0 ARGS...`, or as podvm, with
//! the variables of `podvm_dispatch` set.

use std::os::unix::process::CommandExt;

fn main() {
    podvm_dispatch::main_if_entered("podvm-dispatch-run");
    let mut args = std::env::args_os().skip(1);
    let (Some(root), Some(exe)) = (args.next(), args.next()) else {
        eprintln!("usage: podvm-dispatch-run ROOT GUEST-PATH ARGV0 [ARGS...]");
        std::process::exit(2);
    };
    if let Err(e) = podvm_dispatch::available() {
        eprintln!("podvm-dispatch-run: {e}");
        std::process::exit(125);
    }
    let rest: Vec<std::ffi::OsString> = args.collect();
    let me = std::env::current_exe().expect("/proc/self/exe");
    let err = CommandExt::exec(
        std::process::Command::new(me)
            .arg0(rest.first().cloned().unwrap_or_else(|| exe.clone()))
            .args(rest.iter().skip(1))
            .env(podvm_dispatch::ROOT_VAR, root)
            .env(podvm_dispatch::EXE_VAR, exe),
    );
    eprintln!("podvm-dispatch-run: exec: {err}");
    std::process::exit(126);
}
