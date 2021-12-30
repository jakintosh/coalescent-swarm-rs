use coalescent_swarm::networking;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // connect to a running daemon
    let ipc_server_ip = [127, 0, 0, 1];
    let ipc_server_port = 5000;
    let ipc_server_address = SocketAddr::from((ipc_server_ip, ipc_server_port));
    let handle = match networking::connect_ws(ipc_server_address).await {
        Ok(handle) => {
            println!("new connection with uid: {:?}", handle.uid);
            handle
        }
        Err(err) => {
            println!("couldn't connect: {:?}", err);
            return;
        }
    };

    // send some commands to test how it works
    let request = networking::WireMessage::ApiCall {
        function: "test".to_owned(),
        data: vec![0u8, 0u8, 0u8, 0u8],
    };
    match handle.send(request) {
        Ok(_) => {
            println!("message sent successfully")
        }
        Err(err) => {
            println!("message failed with error: {:?}", err)
        }
    }

    // sleep so it sends
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
}
