use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use crate::common::{self, runbash};
use crate::info_objects::{InfoProvider, SystemStateVar};

pub struct MonitorInfo{
    monitors_plugged_in:HashMap<String,bool>,
    num_monitors_connected:i64,
    last_monitor_changed:String,
    last_action_was_connect:bool
}

impl MonitorInfo{
    pub fn new_refcell() -> Rc<RefCell<MonitorInfo>>{
        Rc::new(
            RefCell::new(
                MonitorInfo {monitors_plugged_in:HashMap::new(),last_action_was_connect: false,last_monitor_changed:String::new(),num_monitors_connected:0}
        ))
    }

    fn update_self(&mut self) -> Result<(),String>{

        let last_conns = self.monitors_plugged_in.clone();

        //oh the shame of writing an entire program, only to run bash commands under the hood ;_;
        let xrandr_info = match runbash("xrandr -q") {
            Ok(v) => {v},
            Err(e) => {return Err(e.to_string())},
        };

        let info_string = String::from_iter(xrandr_info.stdout.iter().map(|x| char::from_u32(*x as u32).unwrap()));

        let status_strings = info_string
                            .lines()
                            .filter(|x| x.contains("connected"))
                            .map(|x| x.split_whitespace().collect::<Vec<&str>>()[..2].join(" "))
                            .collect::<Vec<String>>();

        for status in status_strings{
            let split = status.split_whitespace().collect::<Vec<&str>>();
            self.monitors_plugged_in.insert(
                String::from(*split.get(0).unwrap()), 
                match *split.get(1).unwrap(){
                    "connected" => {true},
                    _ => {false}
                });
        }

        let conn:i64 = self.monitors_plugged_in
            .values()
            .map(|x| if *x {1} else {0})
            .sum();
        self.num_monitors_connected = conn;
        
        let conn_previous:i64 = last_conns
            .values()
            .map(|x| if *x {1} else {0})
            .sum();
        
        if conn_previous < conn{
            self.last_action_was_connect = true;
            for (key,val) in last_conns{
                if val{
                    continue;
                }
                if *self.monitors_plugged_in.get(&key).unwrap(){
                    self.last_monitor_changed = key.to_owned();
                    break;
                }
            }
        }
        else if conn_previous > conn{
            self.last_action_was_connect = false;
            for (key,val) in self.monitors_plugged_in.iter(){
                if *val{
                    continue;
                }
                if *last_conns.get(key).unwrap(){
                    self.last_monitor_changed = key.to_owned();
                    break;
                }
            }
        }
        Ok(())
    }


}

impl InfoProvider for MonitorInfo{
    fn get_info(&mut self) -> Result<HashMap<String,SystemStateVar>,String>{
        self.update_self()?;

        let mut retmap = HashMap::new();
        for key in self.monitors_plugged_in.keys(){
            let v = self.monitors_plugged_in.get(key).unwrap();
            retmap.insert(format!("{}_connected",key), SystemStateVar::Bool(*v));
        }

        retmap.insert(
            String::from("num_monitors_plugged_in"),
            SystemStateVar::Int(self.num_monitors_connected)
        );
        retmap.insert(
            String::from("last_action_was_connect"),
            SystemStateVar::Bool(self.last_action_was_connect)
        );
        retmap.insert(
            String::from("last_monitor_changed"),
            SystemStateVar::String(self.last_monitor_changed.to_owned())
        );
        
        Ok(retmap)
    }
    fn get_name(&self) -> String{
        String::from("MonitorInfo")
    }
}