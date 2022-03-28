use std::convert::TryFrom;
use std::fs::read_to_string;
use crate::interface::InterfaceType;

pub fn get_interface_type(if_name: String) -> InterfaceType {
    let if_type_path: String = format!("/sys/class/net/{}/type", if_name);
    let r = read_to_string(if_type_path);
    match r {
        Ok(content) => {
            let if_type_string = content.trim().to_string();
            match if_type_string.parse::<u32>() {
                Ok(if_type) => {
                    return InterfaceType::try_from(if_type).unwrap_or(InterfaceType::Unknown);
                },
                Err(_) => {
                    return InterfaceType::Unknown;
                }
            }
        },
        Err(_) => {
            return InterfaceType::Unknown;
        }
    };   
}
