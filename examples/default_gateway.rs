use default_net;

fn main(){
    match default_net::get_default_gateway() {
        Ok(default_gateway) => {println!("Default Gateway: {}",default_gateway)},
        Err(e) => {println!("{}",e)},
    }
}
