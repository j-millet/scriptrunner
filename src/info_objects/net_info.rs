use crate::info_objects::{InfoProvider, SystemStateVar};
use crate::common;

use std::collections::HashMap;
use std::fs;

use std::cell::RefCell;
use std::rc::Rc;

pub struct NetInfo{
    interfaces: Vec<String>,
    up_interfaces: Vec<String>,
    last_updated_interface: String
}

impl NetInfo{
    pub fn new_refcell() -> Rc<RefCell<dyn InfoProvider>> {
        let me = NetInfo {
            interfaces: Vec::new(),
            up_interfaces: Vec::new(),
            last_updated_interface: String::new()
        };
        Rc::new(RefCell::new(me))
    }
    fn update_self(&mut self) -> Result<(),String>{
        let net_dir = "/sys/class/net";
        let mut up_int: Vec<String> = Vec::new();
        let mut int:Vec<String> = Vec ::new();

        let read_dir =  match fs::read_dir(net_dir) {
            Ok(res) => {res},
            Err(_) => {
                return Err(String::from("No dir /sys/class/net"))
            },
        };

        for entry in read_dir{
            let interface_path = entry.expect("a").path();
            let interface_name = interface_path.file_name().unwrap().to_str().unwrap().to_string();

            let interface_operstate_path: String = common::join_path(interface_path.to_str().unwrap(), "operstate");
            
            let status_string = match fs::read_to_string(&interface_operstate_path) {
                Ok(res) => {res.to_ascii_lowercase()},
                Err(_) => {
                    return Err(String::from("Some interface is incompatible :/"))
                },
            };
            
            int.push(interface_name.clone());
            
            if status_string.contains("up"){
                up_int.push(
                    interface_name
                );
            }
        }
        //this kinda sucks (slow) but oh well
        //TODO fix :)
        if up_int.len() < self.up_interfaces.len(){
            for int in self.up_interfaces.iter(){
                if !up_int.contains(int){
                    self.last_updated_interface = int.clone();
                    break;
                }
            }
        }else if up_int.len() > self.up_interfaces.len() {
            for int in up_int.iter(){
                if !self.up_interfaces.contains(int){
                    self.last_updated_interface = int.clone();
                    break;
                }
            }
        }
        self.up_interfaces = up_int;
        self.interfaces = int;
        Ok(())
    }

}

impl InfoProvider for NetInfo{
    fn get_name(&self) -> String {
        "NetInfo".to_string()
    }

    fn get_info(&mut self) -> Result<HashMap<String,SystemStateVar>,String> {
        
        self.update_self()?;

        let mut retmap:HashMap<String,SystemStateVar> = HashMap::new();

        retmap.insert(String::from("num_up_interfaces"), SystemStateVar::Int(self.up_interfaces.len() as i64));
        retmap.insert(String::from("last_updated_interface"), SystemStateVar::String(self.last_updated_interface.clone()));

        Ok(retmap)
    }
}