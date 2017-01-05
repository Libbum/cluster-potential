use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::env;

fn read_cluster() -> Result<String, std::io::Error> {
    let mut f = try!(File::open("cluster.xyz"));
    let mut s = String::new();
    try!(f.read_to_string(&mut s));  // `s` contains the contents of "cluster.xyz"
    Ok(s)
}

fn main() {
    //By default we generate values for node 1, although we can use a CLA to build other nodes (ultimately we need 1 - 30).
    let mut node = 1;
    if let Some(arg1) = env::args().nth(1) {
        node = arg1.parse().unwrap();
    }

    let cpus = 30;
    let numxi = 2;
    let numyi = 2;
    let numzi = 300;
    let a = 0.0035;

    let cluster = read_cluster().unwrap();

    println!("Building potential file for node: {}", node);

    let a2 = a/2.0;
    let distnumz = numzi/cpus;
    let numx = numxi as f32;
    let numy = numyi as f32;
    let numz = numzi as f32;
    let grx = numx*a2-a2;
    let gry = numy*a2-a2;
    let grz = numz*a2-a2;

    //For now just try one input
    let xx = 1;
    let yy = 2;
    let zz = 4;

    let tx = -(grx+3.0*a)+(xx as f32)*(2.0*grx)/(numx-1.0);
    let ty = -(gry+3.0*a)+(yy as f32)*(2.0*gry)/(numy-1.0);
    let tz = -(grz+3.0*a)+((zz as f32)+((node as f32)-1.0)*(distnumz as f32))*(2.0*grz)/(numz-1.0);
    let current = format!("O   {:.5}   {:.5}   {:.5}", tx, ty, tz);
    //
    let input_gin = "conp opti\ncart\n".to_string() + &cluster + &current + "\nlibrary streitzmintmire\n";
    // Spawn gulp
    // Should actually set environment from the outside. But we can also do this
    let gulp = match Command::new("./gulp_1")
        .env("GULP_LIB", "/mnt/turtle/Aus/RMIT/gulp-4.4/Libraries/")
        .env("GULP_DOC", "/mnt/turtle/Aus/RMIT/gulp-4.4/Docs/")
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
        Ok(_) => println!("sent the following to gulps stdin: {}", input_gin),
    }

    // Because `stdin` does not live after the above calls, it is `drop`ed,
    // and the pipe is closed.
    //
    // This is very important, otherwise `gulp` wouldn't start processing the
    // input we just sent.

    // The `stdout` field also has type `Option<ChildStdout>` so must be unwrapped.
    let mut output = String::new();
    match gulp.stdout.unwrap().read_to_string(&mut output) {
        Err(why) => panic!("couldn't read gulp stdout: {}", why.description()),
        Ok(_) => print!("gulp responded with:\n{}", output),
    }
}
