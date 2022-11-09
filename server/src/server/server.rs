use std::net::UdpSocket;

#[derive(Debug)]
pub struct Server {
    socket: UdpSocket,
}

impl Server {
    pub fn new(ip: String, port: String) -> Server {
        let address = ip + &port;
        let socket = UdpSocket::bind(&address).unwrap();
        println!("Server up at {}", address);
        Server { socket }
    }

    pub fn listen(&self) {
        println!("Listen to new messages...");
        let mut buf = [0; 1024];
        let (_, from) = self.socket.recv_from(&mut buf).unwrap();
        // Aqui deberian llegar paquetes que pidan o saquen puntos.
        // Armar una seccion critica y/o transacciones quizas.
    }
}
