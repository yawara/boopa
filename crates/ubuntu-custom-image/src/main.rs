use std::{env, path::PathBuf, process};

use ubuntu_custom_image::{BuildRequest, build};

fn main() {
    if let Err(error) = real_main() {
        eprintln!("{error:#}");
        process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("build") => {
            let mut base_iso_path = None;
            let mut manifest_path = None;
            let mut output_path = None;
            let mut work_dir = None;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--base-iso" => base_iso_path = args.next().map(PathBuf::from),
                    "--manifest" => manifest_path = args.next().map(PathBuf::from),
                    "--output" => output_path = args.next().map(PathBuf::from),
                    "--work-dir" => work_dir = args.next().map(PathBuf::from),
                    other => anyhow::bail!("unknown argument: {other}"),
                }
            }

            let request = BuildRequest {
                base_iso_path: base_iso_path
                    .ok_or_else(|| anyhow::anyhow!("missing required argument --base-iso"))?,
                manifest_path: manifest_path
                    .ok_or_else(|| anyhow::anyhow!("missing required argument --manifest"))?,
                output_path: output_path
                    .ok_or_else(|| anyhow::anyhow!("missing required argument --output"))?,
                work_dir,
            };

            let outcome = build(&request)?;
            println!(
                "built custom ISO at {} (metadata {})",
                outcome.output_path.display(),
                outcome.metadata_path.display()
            );
            Ok(())
        }
        _ => {
            eprintln!(
                "usage: cargo run -p ubuntu-custom-image -- build --base-iso <path> --manifest <path> --output <path> [--work-dir <path>]"
            );
            process::exit(2);
        }
    }
}
