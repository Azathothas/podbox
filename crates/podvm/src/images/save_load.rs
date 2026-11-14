use super::*;

pub const SAVE_USAGE: &str = "\
usage: podvm save [--output FILE] <image>

  -o, --output F  write the tarball to F instead of stdout.

  Write one image as an OCI-layout tarball: oci-layout, index.json and
  the blobs the record names, each verified on the way out. The tarball
  goes to stdout (or F); human chatter to stderr, so the stream composes.
  `podvm load` reads it back with every blob verified on the way in
  (TODO/image.md T-1320).
";

pub const LOAD_USAGE: &str = "\
usage: podvm load [--input FILE]

  -i, --input F   read the tarball from F instead of stdin.

  Read an OCI-layout tarball `podvm save` wrote: every blob is hashed
  against its descriptor before it is committed, and the record is
  registered under the tarball's own ref-name annotation. A tarball whose
  bytes changed in transit is refused the way a bad pull is
  (TODO/image.md T-1320).
";

pub const IMPORT_USAGE: &str = "\
usage: podvm import <rootfs.tar> [REPOSITORY[:TAG]]

  Build a runnable record from a plain rootfs tar: the tar becomes the
  image's only layer, and the config and manifest are synthesized around
  its digest. A compressed file is refused by name, because the record's
  media type promises a plain tar. Unnamed imports are recorded as
  `imported:latest` through `Reference::parse`'s own central default
  (TODO/image.md T-1320).
";

/// What `save` was asked for.
pub(super) struct SaveArgs {
    pub(super) output: Option<String>,
    pub(super) want: String,
}

pub(super) fn parse_save(args: &[String]) -> std::result::Result<SaveArgs, i32> {
    let mut output = None;
    let mut want: Option<String> = None;
    let mut expect_output = false;
    for a in args {
        if expect_output {
            output = Some(a.clone());
            expect_output = false;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{SAVE_USAGE}");
                return Err(0);
            }
            "-o" | "--output" => expect_output = true,
            other if other.starts_with("--output=") => {
                output = Some(other["--output=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                crate::parity::admit("save", other, SAVE_USAGE)?;
                return Err(crate::parity::no_arm("save", other));
            }
            other if want.is_none() => want = Some(other.to_string()),
            other => {
                eprintln!("podvm save: {other:?}: save takes one image");
                return Err(EXIT_CLI_ERROR);
            }
        }
    }
    if expect_output {
        eprintln!("podvm save: --output needs a file");
        return Err(EXIT_FLAG_ERROR);
    }
    let Some(want) = want else {
        eprint!("{SAVE_USAGE}");
        return Err(EXIT_CLI_ERROR);
    };
    Ok(SaveArgs { output, want })
}

/// What `load` was asked for.
pub(super) struct LoadArgs {
    pub(super) input: Option<String>,
}

pub(super) fn parse_load(args: &[String]) -> std::result::Result<LoadArgs, i32> {
    let mut input = None;
    let mut expect_input = false;
    for a in args {
        if expect_input {
            input = Some(a.clone());
            expect_input = false;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{LOAD_USAGE}");
                return Err(0);
            }
            "-i" | "--input" => expect_input = true,
            other if other.starts_with("--input=") => {
                input = Some(other["--input=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                crate::parity::admit("load", other, LOAD_USAGE)?;
                return Err(crate::parity::no_arm("load", other));
            }
            other => {
                eprintln!("podvm load: {other:?}: load takes no image argument, only --input");
                return Err(EXIT_CLI_ERROR);
            }
        }
    }
    if expect_input {
        eprintln!("podvm load: --input needs a file");
        return Err(EXIT_FLAG_ERROR);
    }
    Ok(LoadArgs { input })
}

/// What `import` was asked for.
pub(super) struct ImportArgs {
    pub(super) tar: String,
    pub(super) reference: Option<String>,
}

pub(super) fn parse_import(args: &[String]) -> std::result::Result<ImportArgs, i32> {
    let mut positionals: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{IMPORT_USAGE}");
                return Err(0);
            }
            other if other.starts_with('-') => {
                crate::parity::admit("import", other, IMPORT_USAGE)?;
                return Err(crate::parity::no_arm("import", other));
            }
            other => positionals.push(other.to_string()),
        }
    }
    if positionals.len() > 2 {
        eprintln!("podvm import: import takes a tarball and at most one name");
        return Err(EXIT_CLI_ERROR);
    }
    let mut it = positionals.into_iter();
    let Some(tar) = it.next() else {
        eprint!("{IMPORT_USAGE}");
        return Err(EXIT_CLI_ERROR);
    };
    let reference = it.next();
    Ok(ImportArgs { tar, reference })
}

pub fn save(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, SAVE_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, SAVE_USAGE) {
        return c;
    }
    let o = match parse_save(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    // ⛔ stdout carries the tarball and nothing else where no file was
    // given, so the stream composes. T-0110 settled that channel contract.
    if let Some(path) = o.output {
        let mut file = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("podvm save: {path}: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        match podvm_image::layout::save(&store, &o.want, &mut file) {
            Ok(done) => {
                eprintln!(
                    "podvm save: saved {} as {} blobs ({} bytes), digest {}",
                    o.want,
                    done.blobs,
                    podvm_image::space::mib(done.bytes),
                    done.record.manifest_digest
                );
                println!("{path}");
                0
            }
            Err(e) => {
                eprintln!("podvm save: {e}");
                // ⚠ A half-written tarball is a trap for the next load, so a
                // failed save removes what it started.
                let _ = std::fs::remove_file(&path);
                e.exit_code() // T-1842, issue 356: a missing image is 1
            }
        }
    } else {
        let mut out = std::io::stdout().lock();
        match podvm_image::layout::save(&store, &o.want, &mut out) {
            Ok(done) => {
                eprintln!(
                    "podvm save: saved {} as {} blobs ({} bytes), digest {}",
                    o.want,
                    done.blobs,
                    podvm_image::space::mib(done.bytes),
                    done.record.manifest_digest
                );
                0
            }
            Err(e) => {
                eprintln!("podvm save: {e}");
                e.exit_code() // T-1842, issue 356: a missing image is 1
            }
        }
    }
}

pub fn load(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, LOAD_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, LOAD_USAGE) {
        return c;
    }
    let o = match parse_load(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    // ⚠ stdin spills to a file first: `load` needs a path, and a tarball
    // is streamed, never held fully in memory for the copy.
    // ⛔ PodUtils/podvm#336: the spool is a staging file of the store (mode
    // 0700 root, a name unique per call, locked against the sweep), never a
    // name of `$TMPDIR` that another account can plant a link at or read.
    let file_arg: Option<std::path::PathBuf> = o.input.map(std::path::PathBuf::from);
    let mut spool: Option<(std::path::PathBuf, podvm_image::store::StagedFile)> = None;
    let src: std::path::PathBuf = match &file_arg {
        Some(p) => p.clone(),
        None => {
            let (path, mut file) = match store.stage("load-stdin") {
                Ok(s) => s,
                Err(e) => return fail(e),
            };
            let mut stdin = std::io::stdin().lock();
            if let Err(e) = std::io::copy(&mut stdin, &mut file) {
                eprintln!("podvm load: stdin does not read: {e}");
                let _ = std::fs::remove_file(&path);
                return EXIT_RUNTIME_ERROR;
            }
            spool = Some((path.clone(), file));
            path
        }
    };
    let code = match podvm_image::layout::load(&store, &src) {
        Ok(record) => {
            println!("Loaded image: {}", loaded_name(&record));
            0
        }
        Err(e) => {
            eprintln!("podvm load: {e}");
            e.exit_code() // TODO/image.md T-1842
        }
    };
    if let Some((path, file)) = spool {
        drop(file);
        let _ = std::fs::remove_file(&path);
    }
    code
}

pub fn import(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, IMPORT_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, IMPORT_USAGE) {
        return c;
    }
    let o = match parse_import(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    match podvm_image::layout::import(&store, std::path::Path::new(&o.tar), o.reference.as_deref())
    {
        Ok(record) => {
            println!("Loaded image: {}", loaded_name(&record));
            0
        }
        Err(e) => {
            eprintln!("podvm import: {e}");
            e.exit_code() // TODO/image.md T-1842: an invalid name is 1
        }
    }
}

/// The `Loaded image:` line both verbs print: the ref-name where one was
/// recorded, the manifest digest where none was.
fn loaded_name(record: &podvm_image::Record) -> String {
    match &record.tag {
        Some(tag) => format!("{}:{tag}", record.repository),
        None => record.manifest_digest.clone(),
    }
}

pub const VERIFY_USAGE: &str = "\
usage: podvm verify [image|all]

  No argument (or `all`): hash every indexed blob of every record against
  its digest. One image: print its provenance first, then sweep its blobs.

  One line per mismatch plus a summary, and a non-zero exit where any
  blob fails. Verify reports; it never refetches (TODO/image.md T-1321).
";

/// What `verify` was asked for.
pub(super) struct VerifyArgs {
    pub(super) want: Option<String>,
}

pub(super) fn parse_verify(args: &[String]) -> std::result::Result<VerifyArgs, i32> {
    let mut positionals: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{VERIFY_USAGE}");
                return Err(0);
            }
            other if other.starts_with('-') => {
                crate::parity::admit("verify", other, VERIFY_USAGE)?;
                return Err(crate::parity::no_arm("verify", other));
            }
            other => positionals.push(other.to_string()),
        }
    }
    if positionals.len() > 1 {
        eprintln!("podvm verify: verify takes one image or `all`");
        return Err(EXIT_CLI_ERROR);
    }
    Ok(VerifyArgs {
        want: positionals.into_iter().next(),
    })
}

pub fn verify(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, VERIFY_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, VERIFY_USAGE) {
        return c;
    }
    let o = match parse_verify(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let records: Vec<podvm_image::Record> = match o.want.as_deref() {
        None | Some("all") => match store.list() {
            Ok(r) => r,
            Err(e) => return fail(e),
        },
        Some(want) => match store.find_one(want) {
            Ok(r) => vec![r],
            Err(e) => {
                eprintln!("podvm verify: {e}");
                return e.exit_code();
            }
        },
    };
    let single = o.want.as_deref().is_some_and(|w| w != "all");
    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut code = 0;
    for record in &records {
        if single {
            print_provenance(&store, record);
        }
        let name = match &record.tag {
            Some(tag) => format!("{}:{tag}", record.repository),
            None => record.repository.clone(),
        };
        let digests: Vec<String> = record.blobs().iter().map(|s| s.to_string()).collect();
        match store.verify_blobs(&digests) {
            Ok(hits) => {
                checked += digests.len();
                mismatched += hits.len();
                for hit in &hits {
                    println!("MISMATCH {name} {}: computed {}", hit.want, hit.got);
                }
                if hits.is_empty() {
                    println!("OK {name} ({} blobs)", digests.len());
                } else {
                    code = EXIT_RUNTIME_ERROR;
                }
            }
            Err(e) => {
                eprintln!("podvm verify: {e}");
                code = EXIT_RUNTIME_ERROR;
            }
        }
    }
    println!("verify: {checked} blobs checked, {mismatched} mismatched");
    code
}
