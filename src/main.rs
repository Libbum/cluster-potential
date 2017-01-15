extern crate regex;
extern crate getopts;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::path::Path;
use std::env;

use regex::Regex;
use getopts::Options;

fn read_file<P: AsRef<Path>>(file_path: P) -> Result<String, std::io::Error> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn print_usage(program: &str, opts: Options) {
    println!("{}", opts.usage(&format!("Usage: {} [options] <node>", program)));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this usage message.");
    opts.optflag("r", "restart", "Restart from an existing (unfinished) potential_{node}.dat file.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => { m }
        Err(err) => {
            println!("Error parsing options: {}", err);
            std::process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    // By default we generate values for node 1, although we can use a CLA to build other nodes
    let node = if matches.free.is_empty() {
        1
    } else {
        match matches.free[0].parse::<u32>() {
            Ok(n) => n,
            Err(err) => {
                println!("Could not parse node number: {}", err);
                std::process::exit(1);
            }
        }
    };

    //TODO: We could throw these into options if we wanted.
    let cpus = 30;
    let numxi = 300;
    let numyi = 300;
    let numzi = 300;
    let a = 0.0035;

    let cluster = match read_file("cluster.xyz") {
        Ok(s) => s,
        Err(err) => panic!("Cannot get cluster info. Reason: {}.", err.description())
    };

    // setup output file
    let potname = format!("potential_{}.dat", node);
    let potpath = Path::new(&potname);
    let mut potfile = match File::create(&potpath) {
        Err(why) => {
            panic!("couldn't create {}: {}", potpath.display(), why.description())
        }
        Ok(file) => file,
    };

    let re_final = Regex::new(r"Final energy =\s+(-?\d+\.?\d+)\s+eV").unwrap();

    println!("Building potential file for node: {}", node);

    let a2 = a / 2.0;
    let distnumz = numzi / cpus;
    let numx = numxi as f32;
    let numy = numyi as f32;
    let numz = numzi as f32;
    let grx = numx * a2 - a2;
    let gry = numy * a2 - a2;
    let grz = numz * a2 - a2;

    let mut input_gin = String::from("conp opti\n");

    for xx in 0..numxi + 5 + 1 {
        for yy in 0..numyi + 5 + 1 {
            for zz in 0..distnumz + 5 + 1 {

                let tx = -(grx + 3.0 * a) + (xx as f32) * (2.0 * grx) / (numx - 1.0);
                let ty = -(gry + 3.0 * a) + (yy as f32) * (2.0 * gry) / (numy - 1.0);
                let tz = -(grz + 3.0 * a) +
                    ((zz as f32) + ((node as f32) - 1.0) * (distnumz as f32)) * (2.0 * grz) /
                    (numz - 1.0);
                let current = format!("O   {:.5}   {:.5}   {:.5}", tx, ty, tz);

                input_gin.push_str("cart\n");
                input_gin.push_str(&cluster);
                input_gin.push_str(&current);
                input_gin.push_str("\nlibrary streitzmintmire\n\n");
            }

            // Spawn gulp
            let gulp = match Command::new("./gulp")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn() {
                    Err(why) => panic!("couldn't spawn gulp: {}", why.description()),
                    Ok(gulp) => gulp,
                };

            // Write a string to the `stdin` of `gulp`.
            // `stdin` has type `Option<ChildStdin>`, but since we know this instance
            // must have one, we can directly `unwrap` it.
            match gulp.stdin.unwrap().write_all(input_gin.as_bytes()) {
                Err(why) => panic!("couldn't write to gulp stdin: {}", why.description()),
                Ok(_) => {}
            }

            // Because `stdin` does not live after the above calls, it is `drop`ed,
            // and the pipe is closed.
            //
            // This is very important, otherwise `gulp` wouldn't start processing the
            // input we just sent.

            // The `stdout` field also has type `Option<ChildStdout>` so must be unwrapped.
            let mut clust_gout = String::new();
            match gulp.stdout.unwrap().read_to_string(&mut clust_gout) {
                Err(why) => panic!("couldn't read gulp stdout: {}", why.description()),
                Ok(_) => {}
            }

            for cap in re_final.captures_iter(&clust_gout) {
                let potval: Option<f64> = cap.get(1).and_then(|s| s.as_str().parse().ok());
                match potval {
                    Some(p) => {
                        let potout = format!("{:.6}\n", p * 239.2311f64);
                        match potfile.write_all(potout.as_bytes()) {
                            Err(why) => panic!("couldn't write to output: {}", why.description()),
                            Ok(_) => {}
                        }
                    }
                    None => panic!("Issue capturing a final energy from gulp output."),
                }
            }

            // Resetting the string this way should keep it's capacity
            input_gin.clear();
            input_gin.push_str("conp opti\n");
        }
    }

    println!("potential_{}.dat constructed seccesfully.", node);
}
