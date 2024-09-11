use std::process::exit;
use clap::{ command, Arg, ArgAction};
use std::env;

use info_objects::{InfoPublisher, SystemStateVar};

mod info_objects;
mod common;

fn print_keys(){
    let mut providers = info_objects::utils::get_all_info_providers();
    println!("Usable Keys\n{}\n","▼".repeat("Usable Keys".len()));
    for provider in providers.drain(..){
        let name = provider.borrow().get_name();
        let info = provider.try_borrow_mut().unwrap().get_info();
        println!("{}\n{}",name,"-".repeat(name.len()));
        match info{
            Ok(i) => {
                for var in i.keys(){
                    println!("  -{} : {}",match i.get(var).unwrap() {
                        SystemStateVar::String(_) => {"String "},
                        SystemStateVar::Int(_) => {"Integer"},
                        SystemStateVar::Float(_) => {"Float  "},
                        SystemStateVar::Bool(_) => {"Boolean"}
                    },var);
                }
            }
            Err(err) => {
                println!("Module does not work: {}",err);
            }
        }
        println!("");
    }
}

fn main() {
    let matches = command!()
    .arg(
        Arg::new("config-file").short('c').long("config").required(false)
    )
    .arg(
        Arg::new("display-keys").short('v').long("display-keys").required(false).action(ArgAction::SetTrue)
    )
    .get_matches();

    if matches.get_flag("display-keys"){
        print_keys();
        exit(0);
    }

    let config_file_path = match matches.get_one::<String>("config-file"){
        Some(v) => {v}
        None => {"config"}//&common::join_path(std::env::var("HOME").unwrap(), ".config/scriptrunner/config")}
    };

    let mut IP = InfoPublisher::new();

    let mut providers = info_objects::utils::get_all_info_providers();

    for provider in providers.drain(..){
        let name = provider.borrow().get_name();
        match IP.add_provider(provider) {
            Ok(_) => {},
            Err(_) => {
                println!("Provider {} does not work in your environment.",name);
            },
        };
    }

    let mut subs = match info_objects::utils::make_subs_from_config_file(&config_file_path) {
        
        Ok(v) => {v},
        Err(e) => {
            println!("{}, exiting...",e);
            exit(1);
        },
    };

    for sub in subs.drain(..){
        //println!("{:?}",sub);
        match IP.add_subscriber(sub) {
            Ok(_) => {},
            Err(e) => {println!("{}",e)},
        };
    }

    IP.mainloop();
}
