use std::path::PathBuf;
use structopt::StructOpt;

mod error;
mod ips;

/// ips-patch: IPS patch tool
///
/// Applies patch to data read from stdin, writes output to stdout.
#[derive(StructOpt, Debug)]
#[structopt(name = "ips-patch")]
struct Opt {
    #[structopt(name = "FILE", parse(from_os_str))]
    arg_patch: PathBuf,
}

fn main() {
    let args = Opt::from_args();

    match ips::patch(&args.arg_patch) {
        Ok(_) => (),
        Err(e) => {
            use std::io::Write;
            let stderr = std::io::stderr();
            writeln!(&mut stderr.lock(), "{}", e).unwrap();
            std::process::exit(1);
        }
    }
}
