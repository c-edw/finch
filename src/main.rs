mod api;
mod hash;
mod process;

extern crate base64;
extern crate env_logger;
extern crate image;
extern crate rayon;
extern crate reqwest;
extern crate serde;
extern crate walkdir;

#[macro_use]
extern crate log;

#[macro_use]
extern crate structopt;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use rayon::prelude::*;
use structopt::StructOpt;
use walkdir::WalkDir;

use std::env;
use std::path::PathBuf;

#[derive(StructOpt, Debug)]
#[structopt(name = "finch")]
pub struct Opt {
    /// Your Google Vision API key.
    #[structopt(short = "k", long = "api_key")]
    api_key: String,

    /// Similarity tolerance. You can probably leave this alone.
    #[structopt(short = "t", long = "tolerance", default_value = "0.95")]
    tolerance: f32,

    /// Target directory containing images to enhance.
    #[structopt(name = "DIRECTORY", default_value = ".", parse(from_os_str))]
    dir: PathBuf,
}

fn main() {
    let opts = Opt::from_args();

    // Get the current working directory.
    // FATAL: This can fail if the directory does not exist, or is invalid.
    let mut cur = match env::current_dir() {
        Ok(cur) => cur,
        Err(_) => {
            error!("Unable to access the current working directory.");
            return;
        }
    };

    cur.push(&opts.dir);

    WalkDir::new(&cur)
        .into_iter()
        .filter_map(|dir| dir.ok())
        .filter(process::is_supported)
        .filter(process::is_file)
        .map(|dir| dir.path().to_owned())
        // For some reason we can't iterate in parallel over directories, so we do some filtering and then collect into a Vec.
        .collect::<Vec<_>>()
        // Iterate over the collection again, but in parallel.
        .par_iter()
        .for_each(|path| {
            match process::process_file(&path, &opts) {
                Ok(_) => info!("Sucessfully processed {}.", path.display()),
                Err(_) => warn!("Failed to process {}.", path.display())
            }
        });
}
