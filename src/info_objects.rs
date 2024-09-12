use core::time;
use std::env::var;
use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::collections::{HashSet,HashMap};
use std::time::Duration;
use std::thread;
use std::error::Error;

use regex::Regex;

use crate::common;

//specific provider structs
pub mod net_info;
pub mod lid_info;
pub mod monitor_info;

#[derive(Debug)]
#[derive(PartialEq, PartialOrd)]
#[derive(Clone)]
pub enum SystemStateVar{
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64)
}

#[derive(Debug)]
#[derive(Clone)]
pub enum Requirement{
    Change,
    LT(SystemStateVar),
    LE(SystemStateVar),
    GT(SystemStateVar),
    GE(SystemStateVar),
    EQ(SystemStateVar),
    NE(SystemStateVar)
}


pub trait InfoProvider{
    fn get_info(&mut self) -> Result<HashMap<String,SystemStateVar>, String>;
    fn get_name(&self) -> String;
}
#[derive(Debug)]
pub struct InfoSubscriber{
    command:String,
    dependencies:HashMap<String,Requirement>,
    last_time_id:i32
}

impl InfoSubscriber{
    pub fn new_refcell(command:&String, dependencies:&HashMap<String,Requirement>) -> Rc<RefCell<InfoSubscriber>>{
        Rc::new(
            RefCell::new(
                InfoSubscriber {command: command.to_owned(),dependencies:dependencies.to_owned(), last_time_id:-1}
            )
        )
    }

    fn insert_dep(dependencies: &mut HashMap<String,Requirement>, dep:&str, term: &str){
        let var_eq = dep.split_terminator(term).collect::<Vec<&str>>();
        let var = var_eq.get(0).unwrap().trim();
        let eq = var_eq.get(1).unwrap().trim();

        let eq_ssv = match eq.parse::<i64>(){
            Ok(v) => {SystemStateVar::Int(v)},
            Err(_) => {
                match eq.parse::<f64>(){
                    Ok(v) => {SystemStateVar::Float(v)},
                    Err(_) => {
                        match eq.parse::<bool>(){
                            Ok(v) => {SystemStateVar::Bool(v)},
                            Err(_) => {SystemStateVar::String(eq.to_owned())}
                        }
                    }
                }
            }
        };

        match term {
            "==" => {dependencies.insert(String::from(var), Requirement::EQ(eq_ssv));},
            "!=" => {dependencies.insert(String::from(var), Requirement::NE(eq_ssv));},
            "<=" => {dependencies.insert(String::from(var), Requirement::LE(eq_ssv));},
            "<" => {dependencies.insert(String::from(var), Requirement::LT(eq_ssv));},
            ">=" => {dependencies.insert(String::from(var), Requirement::GE(eq_ssv));},
            ">" => {dependencies.insert(String::from(var), Requirement::GT(eq_ssv));},
            _ => {panic!()}
        }
        

    }

    pub fn from_config_line(line: &String) -> Result<Rc<RefCell<InfoSubscriber>>,&str>{
        let line_split = line.split_terminator("=>").collect::<Vec<&str>>();
        if !(line_split.len() == 2){
            return Err("Wrong syntax");
        }
        let command = String::from(line_split.get(1).unwrap().trim());
        let mut dependencies: HashMap<String,Requirement> = HashMap::new();

        for dep in line_split.get(0).unwrap().split_terminator("&&"){
            if dep.contains("$:"){
                let var = dep.trim().strip_prefix("$:").unwrap().trim();
                dependencies.insert(String::from(var),Requirement::Change);
            }
            else{
                let mut matched = false;
                for term in vec!["==","!=","<=",">=","<",">"].iter(){
                    if dep.contains(*term){
                        InfoSubscriber::insert_dep(&mut dependencies, dep, *term);
                        matched = true;
                        break;
                    }
                }
                if !matched{
                    return Err("Wrong syntax");
                }
            }
        }
        Ok(InfoSubscriber::new_refcell(&command, &dependencies))
    }

    pub fn get_dependent_keys(&self) -> Vec<String>{
        self.dependencies.keys().map(|x| x.clone()).collect::<Vec<String>>()
    }

    fn inject_variable_values(command:&String,system_state:&HashMap<String,SystemStateVar>) -> Result<String,String> {
        println!("{}",command);
        let re = Regex::new(r"(?:\$:)(?<var>[a-zA-Z0-9_-]+)").expect("What");
        let mut cmd_cpy = command.to_owned();
        for cap in re.captures_iter(command) {
            let var_name = &cap["var"];
            let var_val = match system_state.get(var_name) {
                Some(v) => {v},
                None => {return Err(format!("{} : No such key!!!",var_name))},
            };
            let var_val_string = match var_val {
                SystemStateVar::Bool(v) => {v.to_string()},
                SystemStateVar::Int(v) => {v.to_string()},
                SystemStateVar::Float(v) => {v.to_string()},
                SystemStateVar::String(v) => {v.to_string()}
            };
            cmd_cpy = cmd_cpy.replace(&format!("$:{}",var_name), &var_val_string);
        }
        println!("{}",cmd_cpy);
        Ok(cmd_cpy.to_owned())
    }
    pub fn get_notified(&mut self,system_state:HashMap<String,SystemStateVar>,time_id:i32) -> Result<(),String>{
        if time_id == self.last_time_id{
            return Ok(());
        }
        let mut matching = 0;
        let required = self.dependencies.len();

        for key in self.dependencies.keys(){
            let dep = self.dependencies.get(key).unwrap();
            let ss_val = system_state.get(key).unwrap();
            match dep {
                Requirement::Change => {matching += 1;},
                Requirement::EQ(req) => {if ss_val == req{matching += 1;}},
                Requirement::NE(req) => {if ss_val != req{matching+=1;}},
                Requirement::LT(req) => {if ss_val < req{matching += 1;}},
                Requirement::LE(req) => {if ss_val <= req{matching+=1;}},
                Requirement::GT(req) => {if ss_val > req{matching += 1;}},
                Requirement::GE(req) => {if ss_val >= req{matching+=1;}},
            }
        }
        if matching == required{
            common::runbash(&InfoSubscriber::inject_variable_values(&self.command, &system_state)?);
        }

        self.last_time_id = time_id;
        Ok(())
    }
}


pub struct InfoPublisher{
    subscribers: HashMap<String,Vec<Rc<RefCell<InfoSubscriber>>>>, //lord have mercy
    providers: Vec<Rc<RefCell<dyn InfoProvider>>>,
    system_state_map: HashMap<String,SystemStateVar>
}

impl InfoPublisher{

    pub fn new() -> InfoPublisher{
        InfoPublisher {subscribers: HashMap::new(), providers: Vec::new(), system_state_map: HashMap::new()}
    }
    pub fn add_subscriber(&mut self,subscriber:Rc<RefCell<InfoSubscriber>>) -> Result<(),String>{
        let mut dependent_keys = subscriber.borrow().get_dependent_keys();
        for key in dependent_keys.drain(..){
            if !self.subscribers.contains_key(&key){
                return Err(format!("The key '{}' is not provided by any module.",key));
            }
            else{    
                self.subscribers.get_mut(&key).unwrap().push(subscriber.clone());
            }
        }   
        Ok(())
    }

    pub fn add_provider(&mut self, provider:Rc<RefCell<dyn InfoProvider>> ) -> Result<(),&str>{
        match provider.borrow_mut().get_info(){
            Ok(info) => {
                for key in info.keys(){
                    self.subscribers.insert(String::from(key), Vec::new());
                }
                self.providers.push(provider.clone());
                Ok(())
            }
            Err(_) => {
                Err("provider unable to provide information")
            }
        }
        
    }

    pub fn mainloop(&mut self){
        let mut id: i32 = 0;
        loop{
            let mut subs_needing_an_update:Vec<Rc<RefCell<InfoSubscriber>>> = Vec::new();
            for prov in self.providers.iter(){
                let info = prov.borrow_mut().get_info().unwrap();
                for key in info.keys(){
                    if !self.subscribers.contains_key(key){
                        continue;
                    }
                    if(!self.system_state_map.contains_key(key) || (self.system_state_map.get(key).unwrap() != info.get(key).unwrap())){
                        self.system_state_map.insert(key.to_owned(), info.get(key).unwrap().to_owned());
                        subs_needing_an_update = vec![subs_needing_an_update,self.subscribers.get(key).unwrap().to_owned()].concat();
                    }
                }
            }
            for sub in subs_needing_an_update.iter(){
                match sub.borrow_mut().get_notified(self.system_state_map.to_owned(), id) {
                    Ok(_) => {},
                    Err(e) => {println!("{}",e)},
                };
            }
            id = (id+1)%i32::MAX;
            thread::sleep(Duration::from_millis(500));
        }

    }
}

pub mod utils{
    use std::{cell::RefCell, fs, rc::Rc};

    use crate::info_objects::{monitor_info::MonitorInfo, net_info::NetInfo};
    use crate::info_objects::lid_info::LidInfo;

    use super::{InfoProvider, InfoSubscriber};

    pub fn make_subs_from_config_file(filepath: &str) -> Result<Vec<Rc<RefCell<InfoSubscriber>>>,String>{
        let mut subs:Vec<Rc<RefCell<InfoSubscriber>>> = Vec::new();
        let config_file_contents = match fs::read_to_string(filepath) {
            Ok(v) => {v},
            Err(_) => {
                return Err(format!("Config file '{}' does not exist",filepath));
            },
        };

        for line in config_file_contents.lines(){
            if !line.contains("=>"){
                continue;
            }
            subs.push(InfoSubscriber::from_config_line(&String::from(line))?);
        }
        Result::Ok(subs)
    }

    pub fn get_all_info_providers() -> Vec<Rc<RefCell<dyn InfoProvider>>>{
        vec![
            NetInfo::new_refcell(),
            LidInfo::new_refcell(),
            MonitorInfo::new_refcell()
        ]
    }
}