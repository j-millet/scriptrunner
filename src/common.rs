use std::process::{Command, Output};
use std::{io,env};

pub fn join_path<S1: AsRef<str>,S2: AsRef<str>>(p1:S1,p2:S2)->String{
    let mut p1_new = p1.as_ref().to_string();
    p1_new.push_str("/");
    p1_new.push_str(p2.as_ref());
    return p1_new;
}

pub fn runbash(cmd:&str)-> io::Result<Output>{
    Command::new("bash").
        current_dir(env::var("HOME").
        unwrap()).
        arg("-c").
        arg(cmd).
        output()
}