fn main() -> std::process::ExitCode {
    let code = podbox_ssh::cli::main(std::env::args().collect());
    std::process::ExitCode::from(u8::try_from(code).unwrap_or(1))
}
