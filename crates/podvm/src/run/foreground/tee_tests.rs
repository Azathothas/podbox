//! TODO/enter.md T-1700. The stdout tee of a foreground run delivers
//! each byte that the payload wrote before the restore.

use super::*;

/// The bytes that the drain reads from the target pipe until EOF.
fn drain(reader: i64) -> usize {
    let mut buf = [0u8; 65536];
    let mut total = 0usize;
    loop {
        match podvm_probe::sys::read(reader, &mut buf) {
            Ok(0) => return total,
            Ok(n) => total += n as usize,
            Err(e) if e == podvm_probe::sys::EINTR => continue,
            Err(e) => panic!("read of the target pipe: {} ({})", e.name(), e.0),
        }
    }
}

/// The restore must not close the saved descriptor while the pump has
/// bytes to write to it.
///
/// The target is a pipe whose reader starts 200 ms late. The test writes
/// more than the target pipe and one read of the pump hold, so at the
/// restore the pump is blocked in a write and more bytes wait in the tee
/// pipe. A restore that closes the saved descriptor before the join makes
/// the next write of the pump fail with EBADF, and the drain counts fewer
/// bytes than the test wrote. This is the lost output of T-1700.
#[test]
fn untee_delivers_every_byte_before_the_saved_descriptor_closes() {
    let dir = std::env::temp_dir().join(format!("podvm-tee-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("log");

    let mut target = [0i32; 2];
    podvm_probe::sys::pipe2(&mut target, podvm_probe::sys::O_CLOEXEC).unwrap();
    let (reader, writer) = (i64::from(target[0]), i64::from(target[1]));
    let drainer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        drain(reader)
    });

    let tee = tee_fd_to(writer, &log).unwrap();
    const LEN: usize = 160 * 1024;
    let bytes = vec![b'x'; LEN];
    let mut sent = 0usize;
    while sent < LEN {
        match podvm_probe::sys::write(writer, &bytes[sent..]) {
            Ok(n) => sent += n as usize,
            Err(e) if e == podvm_probe::sys::EINTR => continue,
            Err(e) => panic!("write into the tee: {} ({})", e.name(), e.0),
        }
    }
    untee(tee);
    // `writer` holds the target pipe again: its close is the EOF of the
    // drain.
    let _ = podvm_probe::sys::close(writer);
    let got = drainer.join().unwrap();
    let _ = podvm_probe::sys::close(reader);
    let logged = std::fs::metadata(&log).map(|m| m.len()).unwrap_or(0);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(got, LEN, "the target got {got} of {LEN} bytes");
    assert_eq!(logged as usize, LEN, "the log got {logged} of {LEN} bytes");
}
