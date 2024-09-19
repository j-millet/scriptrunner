use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashSet,HashMap};
use std::time::Duration;
use std::thread;
use evalexpr::eval_boolean;

use regex::Regex;

use crate::common;

//specific provider structs
pub mod net_info;
pub mod lid_info;
pub mod display_info;

#[derive(Debug)]
#[derive(PartialEq, PartialOrd)]
#[derive(Clone)]
pub enum SystemStateVar{
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64)
}


pub trait InfoProvider{
    fn get_info(&mut self) -> Result<HashMap<String,SystemStateVar>, String>;
    fn get_name(&self) -> String;
}
#[derive(Debug)]
pub struct InfoSubscriber{
    command:String,
    dependencies:String,
    last_time_id:i32
}

impl InfoSubscriber{
    pub fn new_refcell(command:&String, dependencies:&String) -> Rc<RefCell<InfoSubscriber>>{
        Rc::new(
            RefCell::new(
                InfoSubscriber {command: command.to_owned(),dependencies:dependencies.to_owned(), last_time_id:-1}
            )
        )
    }

    pub fn from_config_line(line: &String) -> Result<Rc<RefCell<InfoSubscriber>>,&str>{
        let line_split = line.split_terminator("=>").collect::<Vec<&str>>();
        if !(line_split.len() == 2){
            return Err("Wrong syntax");
        }
        let command = String::from(line_split.get(1).unwrap().trim());
        let dependencies = String::from(line_split.get(0).unwrap().trim());
        Ok(InfoSubscriber::new_refcell(&command, &dependencies))
    }

    pub fn get_dependent_vars(&self) -> Vec<String>{
        let re = Regex::new(r"(?<var>[a-zA-Z_-]+[a-zA-Z0-9_-]*)").unwrap();
        let mut ret_set = HashSet::new();
        for cap in re.captures_iter(&self.dependencies){
            ret_set.insert(String::from(&cap["var"]));
        }
        ret_set.drain().collect()
    }

    fn eval_dependencies(&self, system_state:&HashMap<String,SystemStateVar>) -> Result<bool, String>{
        let mut dep_cpy = self.dependencies.to_owned();

        let re = Regex::new(r"(?<var>\$:[a-zA-Z0-9_-]+)").unwrap();

        for cap in re.captures_iter(&self.dependencies){
            dep_cpy = dep_cpy.replace(&cap["var"], "true");
        }

        for (key,value) in system_state.iter(){
            dep_cpy = dep_cpy.replace(key, &match value {
                SystemStateVar::Bool(v) => {v.to_string()},
                SystemStateVar::Float(v) => {v.to_string()},
                SystemStateVar::Int(v) => {v.to_string()},
                SystemStateVar::String(v) => {format!("\"{}\"",v)}
            });
        }
        match eval_boolean(&dep_cpy) {
            Ok(v) => {Ok(v)},
            Err(err) => {Err(err.to_string())},
        }
    }

    fn inject_variable_values(command:&String,system_state:&HashMap<String,SystemStateVar>) -> Result<String,String> {
        let re = Regex::new(r"(?:\$:)(?<var>[a-zA-Z0-9_-]+)").expect("What");
        let mut cmd_cpy = command.to_owned();
        for cap in re.captures_iter(command) {
            let var_name = &cap["var"];
            let var_val = match system_state.get(var_name) {
                Some(v) => {v},
                None => {return Err(format!("{} : No such variable!!!",var_name))},
            };
            let var_val_string = match var_val {
                SystemStateVar::Bool(v) => {v.to_string()},
                SystemStateVar::Int(v) => {v.to_string()},
                SystemStateVar::Float(v) => {v.to_string()},
                SystemStateVar::String(v) => {v.to_string()}
            };
            cmd_cpy = cmd_cpy.replace(&format!("$:{}",var_name), &var_val_string);
        }
        Ok(cmd_cpy.to_owned())
    }
    pub fn get_notified(&mut self,system_state:HashMap<String,SystemStateVar>,time_id:i32) -> Result<(),String>{
        if time_id == self.last_time_id{
            return Ok(());
        }
        let run_cmd = self.eval_dependencies(&system_state)?;
        if run_cmd{
            let _ = common::runbash(&InfoSubscriber::inject_variable_values(&self.command, &system_state)?);
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
        let mut dependent_vars = subscriber.borrow().get_dependent_vars();
        for var in dependent_vars.drain(..){
            if !self.subscribers.contains_key(&var){
                return Err(format!("The variable '{}' is not provided by any module.",var));
            }
            else{    
                self.subscribers.get_mut(&var).unwrap().push(subscriber.clone());
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
    //this feels like the wrong thing to do (by that I mean returning a vector AND doing side-effects on the struct in the same function)
    //TODO refactor this
    fn collect(&mut self) -> Vec<Rc<RefCell<InfoSubscriber>>>{
        let mut subs_to_update = Vec::new();
        for prov in self.providers.iter(){
            let info = prov.borrow_mut().get_info().unwrap();
            for key in info.keys(){
                if !self.subscribers.contains_key(key){
                    continue;
                }
                if !self.system_state_map.contains_key(key) || (self.system_state_map.get(key).unwrap() != info.get(key).unwrap()){
                    self.system_state_map.insert(key.to_owned(), info.get(key).unwrap().to_owned());
                    subs_to_update = vec![subs_to_update,self.subscribers.get(key).unwrap().to_owned()].concat();
                }
            }
        }
        subs_to_update
    }
    pub fn mainloop(&mut self){
        self.collect(); //dry collect so as not to react to variables 'changing' on startup
        let mut id: i32 = 0;
        loop{
            
            for sub in self.collect().iter(){
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

    use crate::info_objects::{display_info::MonitorInfo, net_info::NetInfo};
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