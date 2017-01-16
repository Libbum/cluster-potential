extern crate regex;
extern crate getopts;

use std::error::Error;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::path::Path;
use std::env;

use regex::Regex;
use getopts::Options;

fn read_file<P: AsRef<Path>>(file_path: P) -> Result<String, std::io::Error> {
    let mut contents = String::new();
    OpenOptions::new().read(true).open(file_path)?.read_to_string(&mut contents)?;
    Ok(contents)
}

fn get_restart_position(solved: u64, totals: (u32, u32, u32)) -> Result<(u32, u32, u32), String> {
    let (xt, yt, zt) = totals;
    let mut counter: u64 = 0;

    for xx in 0..xt {
        for yy in 0..yt {
            for zz in 0..zt {
                if counter == solved {
                    if zz == zt {
                        //increment y one
                        if yy == yt {
                            //increment x one
                            return Ok((xx+1,0,0));
                        }
                        return Ok((xx,yy+1,0));
                    }
                    return Ok((xx,yy,zz));
                }
                counter += 1;
            }
        }
    }
    Err(From::from("Could not identify restart position."))
}

fn print_usage(program: &str, opts: Options) {
    println!("{}", opts.usage(&format!("Usage: {} [options] <node>", program)));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this usage message.");
    opts.optflag("r", "restart", "Restart from an existing (unfinished) potential_{node}.dat file.
                                  One must be careful not to alter num{x,y,z} during this process.");

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

    let distnumz = numzi / cpus;
    let mut startloop = (0,0,0);

    let cluster = match read_file("cluster.xyz") {
        Ok(s) => s,
        Err(err) => panic!("Cannot get cluster info. Reason: {}.", err.description())
    };

    println!("Building potential file for node: {}", node);

    // setup output file
    let potname = format!("potential_{}.dat", node);
    let mut potfile: std::fs::File;
    if matches.opt_present("r") {
        //We want to restart from a current potential file
        potfile = match OpenOptions::new().read(true).append(true).open(&potname) {
            Err(why) => {
                panic!("Issue with {}: {}", &potname, why.description())
            }
            Ok(file) => file,
        };
        let reader = BufReader::new(&potfile);
        let solved: u64 = reader.lines().count() as u64;
        println!("Current potential has {} of {} points already solved.",
                 solved, (numxi+6)*(numyi+6)*(distnumz+6));
        //Just a note here. It would be awesome if r.l.c was a multiple of distnumz+6,
        //but I doubt it will happen all the time.
        startloop = match get_restart_position(solved, (numxi+6, numyi+6, distnumz+6)) {
            Ok(values) => values,
            Err(why) => panic!("{}", why)
        };
        println!("Starting at position {:?}.", startloop);
    } else {
        //Create a potential file (or truncate the current one)
        potfile = match OpenOptions::new().write(true).create(true).open(&potname) {
            Err(why) => {
                panic!("Couldn't create {}: {}", &potname, why.description())
            }
            Ok(file) => file,
        };
    }

    let re_final = Regex::new(r"Final energy =\s+(-?\d+\.?\d+)\s+eV").unwrap();


    let a2 = a / 2.0;
    let numx = numxi as f32;
    let numy = numyi as f32;
    let numz = numzi as f32;
    let grx = numx * a2 - a2;
    let gry = numy * a2 - a2;
    let grz = numz * a2 - a2;

    let mut input_gin = String::from("conp opti\n");

    for xx in 0..numxi + 5 + 1 {
        for yy in 0..numyi + 5 + 1 {
            if xx >= startloop.0 && yy >= startloop.1 {
                for zz in 0..distnumz + 5 + 1 {
                    if zz >= startloop.2 {
                        startloop = (0,0,0); //turn off restart truncator

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
    }

    println!("potential_{}.dat constructed seccesfully.", node);
}
