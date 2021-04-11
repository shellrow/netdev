use default_net;

fn main(){
    let default_gateway = default_net::get_default_gateway();
    println!("Default Gateway");
    println!("IP {:?}", default_gateway.ip);
    println!("MAC {:?}", default_gateway.mac);
}
