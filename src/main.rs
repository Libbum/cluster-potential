use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::{Command, Stdio};

fn read_input() -> Result<String, std::io::Error> {
    let mut f = try!(File::open("input.gin"));
    let mut s = String::new();
    try!(f.read_to_string(&mut s));  // `s` contains the contents of "input.gin"
    Ok(s)
}


fn main() {
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
    //read_input pulls a file from disk, but we can also build the string instead.
    match gulp.stdin.unwrap().write_all(read_input().unwrap().as_bytes()) {
        Err(why) => panic!("couldn't write to gulp stdin: {}", why.description()),
        Ok(_) => println!("sent input.gin to gulp"),
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
