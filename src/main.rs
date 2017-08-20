#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

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
    OpenOptions::new()
        .read(true)
        .open(file_path)?
        .read_to_string(&mut contents)?;
    Ok(contents)
}

fn get_index_position(solved: u32, totals: (u32, u32, u32)) -> (u32, u32, u32) {
    //Note: This gives the index after solved. Good for restarts,
    //but we may want solved-1 for chunk index.
    let (_, yt, zt) = totals;

    let xx = solved / (yt * zt);
    let r2 = solved % (yt * zt);
    let yy = r2 / zt;
    let zz = r2 % zt;

    (xx, yy, zz)
}

fn print_usage(program: &str, opts: &Options) {
    println!(
        "{}",
        opts.usage(&format!("Usage: {} [options] <node>", program))
    );
}

fn main() {
    let chunk_tot = 48; //If we stick to 306*306*16, 48 gives us a round cut.

    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this usage message.");
    opts.optflag(
        "r",
        "restart",
        "Restart from an existing (unfinished) potential_{node}.dat file.
                                  \
                  One must be careful not to alter num{x,y,z} during this process.",
    );
    opts.optopt(
        "c",
        "chunk",
        format!("Enable chunking and build chunk N of {}.", chunk_tot).as_str(),
        "N",
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(err) => {
            println!("Error parsing options: {}", err);
            std::process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(program, &opts);
        return;
    }

    let mut do_chunk = false;
    let mut chunk_num: u32 = 1;
    if let Some(c) = matches.opt_str("c") {
        do_chunk = true;
        chunk_num = match c.parse::<u32>() {
            Ok(n) => n,
            Err(err) => {
                println!("Could not parse chunk value: {}", err);
                std::process::exit(1);
            }
        };
        if chunk_num > chunk_tot {
            println!(
                "Chunk value: {} is greater than current total chunk number: {}.",
                chunk_num,
                chunk_tot
            );
            std::process::exit(1);
        }
        if chunk_num == 0 {
            println!("Chunk value cannot be 0.");
            std::process::exit(1);
        }
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
    let mut startloop = (0, 0, 0);
    let total_points = (numxi + 6) * (numyi + 6) * (distnumz + 6);
    let loop_tops = (numxi + 6, numyi + 6, distnumz + 6);

    let clusternn = match read_file("clusternn.xyz") {
        Ok(s) => s,
        Err(err) => panic!("Cannot get nn cluster info. Reason: {}.", err.description()),
    };

    let cluster2nn = match read_file("cluster2nn_wo_nn.xyz") {
        Ok(s) => s,
        Err(err) => {
            panic!(
                "Cannot get 2nn cluster info. Reason: {}.",
                err.description()
            )
        }
    };

    println!("Building potential file for node: {}", node);

    let mut curr_chunk_start = 0;
    let mut curr_chunk_end = total_points;
    let mut per_chunk = 0;
    if do_chunk {
        per_chunk = total_points / chunk_tot;
        curr_chunk_start = per_chunk * (chunk_num - 1);
        curr_chunk_end = (per_chunk * (chunk_num)) - 1;
        println!(
            "Current job is for chunk {} of {}. Points per chunk: {}",
            chunk_num,
            chunk_tot,
            per_chunk
        );
        println!(
            "Index at start: {}, index at end: {}.",
            curr_chunk_start,
            curr_chunk_end
        );
    }

    // setup output file
    let potname = if do_chunk {
        format!("potential_{}.c{}.dat", node, chunk_num)
    } else {
        format!("potential_{}.dat", node)
    };
    let mut potfile: std::fs::File;
    if matches.opt_present("r") {
        //We want to restart from a current potential file
        potfile = match OpenOptions::new().read(true).append(true).open(&potname) {
            Err(why) => panic!("Issue with {}: {}", &potname, why.description()),
            Ok(file) => file,
        };
        let reader = BufReader::new(&potfile);
        let solved = reader.lines().count() as u32;
        if do_chunk {
            println!(
                "Current potential chunk has {} of {} points already solved.",
                solved,
                per_chunk
            );
            curr_chunk_start += solved;
            println!("Changed start index to: {}", curr_chunk_start);
        } else {
            println!(
                "Current potential has {} of {} points already solved.",
                solved,
                total_points
            );
            startloop = get_index_position(solved, loop_tops);
            println!("Starting at position {:?}.", startloop);
        }
    } else {
        //Create a potential file (or truncate the current one)
        potfile = match OpenOptions::new().write(true).create(true).open(&potname) {
            Err(why) => panic!("Couldn't create {}: {}", &potname, why.description()),
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
                let mut do_run = false;
                let mut index: u32;
                for zz in 0..distnumz + 5 + 1 {
                    index = loop_tops.1 * loop_tops.2 * xx + loop_tops.2 * yy + zz;
                    if curr_chunk_start <= index && index <= curr_chunk_end && zz >= startloop.2 {
                        startloop = (0, 0, 0); //turn off restart truncator
                        do_run = true;

                        let tx = -(grx + 3.0 * a) + (xx as f32) * (2.0 * grx) / (numx - 1.0);
                        let ty = -(gry + 3.0 * a) + (yy as f32) * (2.0 * gry) / (numy - 1.0);
                        let tz = -(grz + 3.0 * a) +
                            ((zz as f32) + ((node as f32) - 1.0) * (distnumz as f32)) *
                                (2.0 * grz) / (numz - 1.0);
                        let current = format!("O   {:.5}   {:.5}   {:.5}", tx, ty, tz);

                        input_gin.push_str("cart region 1\n");
                        input_gin.push_str(&clusternn);
                        input_gin.push_str("cart region 2 rigid\n");
                        input_gin.push_str(&cluster2nn);
                        input_gin.push_str(&current);
                        input_gin.push_str("\nlibrary streitzmintmire\n\n");
                    }
                }
                if do_run {
                    println!("{}, {}.", xx, yy);
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
                    if let Err(why) = gulp.stdin.unwrap().write_all(input_gin.as_bytes()) {
                        panic!("couldn't write to gulp stdin: {}", why.description());
                    }

                    // Because `stdin` does not live after the above calls, it is `drop`ed,
                    // and the pipe is closed.
                    //
                    // This is very important, otherwise `gulp` wouldn't start processing the
                    // input we just sent.

                    // The `stdout` field also has type `Option<ChildStdout>` so must be unwrapped.
                    let mut clust_gout = String::new();
                    if let Err(why) = gulp.stdout.unwrap().read_to_string(&mut clust_gout) {
                        panic!("couldn't read gulp stdout: {}", why.description());
                    }

                    for cap in re_final.captures_iter(&clust_gout) {
                        let potval: Option<f64> = cap.get(1).and_then(|s| s.as_str().parse().ok());
                        match potval {
                            Some(p) => {
                                let potout = format!("{:.6}\n", p * 239.2311f64);
                                if let Err(why) = potfile.write_all(potout.as_bytes()) {
                                    panic!("couldn't write to output: {}", why.description());
                                }
                            }
                            None => panic!("Issue capturing a final energy from gulp output."),
                        }
                    }
                }
                // Resetting the string this way should keep it's capacity
                input_gin.clear();
                input_gin.push_str("conp opti\n");
            }
        }
    }

    println!("{} constructed seccesfully.", potname);
}
