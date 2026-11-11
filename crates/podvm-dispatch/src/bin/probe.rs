//! A static test payload of the native-static rung. Each verb prints one
//! answer; the tests of `podvm-dispatch` read it.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};

static GOT: AtomicBool = AtomicBool::new(false);

extern "C" fn on_usr1(_: i32) {
    GOT.store(true, Ordering::SeqCst);
}

fn cat(p: &str) -> String {
    let mut s = String::new();
    match std::fs::File::open(p) {
        Ok(mut f) => {
            let _ = f.read_to_string(&mut s);
            s.trim_end().to_string()
        }
        Err(e) => format!("ERR {e}"),
    }
}

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, opt: i32) -> i32;
    fn signal(sig: i32, h: extern "C" fn(i32)) -> usize;
    fn raise(sig: i32) -> i32;
    fn _exit(c: i32) -> !;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verb = args.get(1).map(String::as_str).unwrap_or("");
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    match verb {
        "cat" => println!("{}", cat(&arg(2))),
        "cwd" => println!(
            "{}",
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("ERR {e}"))
        ),
        "cd" => match std::env::set_current_dir(arg(2)) {
            Ok(()) => println!(
                "{} {}",
                std::env::current_dir().unwrap().display(),
                cat(&arg(3))
            ),
            Err(e) => println!("ERR {e}"),
        },
        "exe" => println!(
            "{}",
            std::fs::read_link("/proc/self/exe")
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("ERR {e}"))
        ),
        "ls" => {
            let mut v: Vec<String> = std::fs::read_dir(arg(2))
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            v.sort();
            println!("{}", v.join(" "));
        }
        "write" => {
            let r = std::fs::write(arg(2), arg(3)).and_then(|()| std::fs::rename(arg(2), arg(4)));
            println!(
                "{}",
                match r {
                    Ok(()) => cat(&arg(4)),
                    Err(e) => format!("ERR {e}"),
                }
            );
        }
        "fork-cat" => {
            // SAFETY: the child prints and exits with no other thread.
            let pid = unsafe { fork() };
            if pid == 0 {
                println!("child {}", cat(&arg(2)));
                // SAFETY: exit the child.
                unsafe { _exit(7) };
            }
            let mut st = 0;
            // SAFETY: st lives for the call.
            unsafe { waitpid(pid, &mut st, 0) };
            println!("parent status {}", (st >> 8) & 0xff);
        }
        "thread-cat" => {
            let p = arg(2);
            let got = std::thread::spawn(move || cat(&p)).join().unwrap();
            println!("thread {got}");
        }
        "signal" => {
            // SAFETY: a handler that stores to an atomic.
            unsafe {
                signal(10, on_usr1);
                raise(10);
            }
            println!("usr1 {}", GOT.load(Ordering::SeqCst));
        }
        "exec" => {
            let e = std::process::Command::new(arg(2)).args(&args[3..]).exec();
            println!("ERR {e}");
        }
        "spawn" => {
            let o = std::process::Command::new(arg(2)).args(&args[3..]).output();
            match o {
                Ok(o) => println!(
                    "spawn {} {}",
                    o.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&o.stdout).trim_end()
                ),
                Err(e) => println!("ERR {e}"),
            }
        }
        "alloc" => {
            let v: Vec<u8> = vec![7; 64 << 20];
            println!("alloc {}", v.iter().map(|&b| b as u64).sum::<u64>());
        }
        _ => {
            eprintln!("podvm-dispatch-probe: no verb {verb:?}");
            std::process::exit(2);
        }
    }
}
