use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use crate::info_objects::{InfoProvider, SystemStateVar};


pub struct LidInfo{
    lid_open: bool
}

impl LidInfo{
    pub fn new_refcell() -> Rc<RefCell<LidInfo>>{
        Rc::new(RefCell::new(LidInfo{lid_open: true}))
    }

    fn update_self(&mut self) -> Result<(),String>{
        let contents = match fs::read_to_string("/proc/acpi/button/lid/LID/state") {
            Ok(v) => {v},
            Err(_) => {return Err(String::from("No lid file! (/proc/acpi/button/lid/LID/state)"));},
        };
        match contents.split_whitespace().enumerate().last().unwrap().1{
            "open" => {self.lid_open = true;},
            _ => {self.lid_open = false;}

        }
        Ok(())
    }
}

impl InfoProvider for LidInfo{
    fn get_info(&mut self) -> Result<std::collections::HashMap<String,SystemStateVar>, String> {
        self.update_self()?;
        let mut retmap = HashMap::new();
        retmap.insert(String::from("lid_open"), SystemStateVar::Bool(self.lid_open));
        Ok(retmap)
    }
    fn get_name(&self) -> String {
        String::from("LidInfo")
    }
    fn get_provided_vars(&self) -> Vec<String> {
        vec![
            String::from("lid_open")
        ]
    }
}