use clap::CommandFactory;
use clap_mangen::Man;
use marathon_cloud::cli::Cli;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Result;

/// Man page can be created with:
/// `cargo run --bin marathon-cloud-mangen`
/// in a directory specified by the environment variable OUT_DIR.
/// See <https://doc.rust-lang.org/cargo/reference/environment-variables.html>
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let first_arg: Option<OsString> = args.get(1).map(|s| s.into());
    let out_dir = std::env::var_os("OUT_DIR")
        .or(first_arg)
        .ok_or(std::io::ErrorKind::NotFound)?;
    let out_dir = std::path::PathBuf::from(out_dir);

    let out_path = out_dir.join(format!("{}.1", env!("CARGO_PKG_NAME")));
    let app = Cli::command();
    let man = Man::new(app);
    let mut buffer = Vec::<u8>::new();
    man.render(&mut buffer)?;
    fs::write(&out_path, buffer)?;
    println!("Man page is generated at {out_path:?}");
    Ok(())
}
