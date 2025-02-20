//! RakNet Protocol implementation by Rust.
//! 
//! Raknet is a reliable udp transport protocol that is often used for communication between game clients and servers. This project is an incomplete implementation of the protocol.
//! 
//! Reference : <http://www.jenkinssoftware.com/raknet/manual/index.html>
//! 
//! _This project is not affiliated with Jenkins Software LLC nor RakNet._
//! 
//! # Features
//! 
//! * Async
//! * MIT License
//! * Pure Rust implementation
//! * Selective Retransmission (TCP/Full Retransmission)
//! * Non-delayed ACK (TCP/Delayed ACK)
//! * RTO Not Doubled (TCP/RTO Doubled)
//! * Linux/Windows/Mac/BSD support
//! * Compatible with Minecraft 1.18.x
//! 
//! # Get Started
//! 
//! ```toml
//! # Cargo.toml
//! [dependencies]
//! rust-raknet = "0.5.1"
//! ```
//! 
//! # Reliability
//! 
//! - [x] unreliable
//! - [x] unreliable sequenced
//! - [x] reliable
//! - [x] reliable ordered
//! - [x] reliable sequenced

//! # Example

//! ```ignore
//! //server
//! 
//! async fn serve(){
//!     let mut listener = RaknetListener::bind("127.0.0.1:19132".parse().unwrap()).await.unwrap();
//!     listener.listen().await;
//!     loop{
//!         let mut socket = listener.accept().await.unwrap();
//!         let buf = socket.recv().await.unwrap();
//!         if buf[0] == 0xfe{
//!             //do something
//!         }
//!         socket.close().await.unwrap();
//!     }
//! }
//! 
//! ```
//! 
//! ```ignore
//! //client
//! 
//! async fn connect(){
//!     let mut socket = RaknetSocket::connect("127.0.0.1:19132".parse().unwrap()).await.unwrap();
//!     socket.send(&[0xfe], Reliability::ReliableOrdered).await.unwrap();
//!     let buf = socket.recv().await.unwrap();
//!     if buf[0] == 0xfe{
//!         //do something
//!     }
//!     socket.close().await.unwrap();
//! }
//! ```

mod socket;
mod packet;
mod utils;
mod datatype;
mod arq;
mod fragment;
mod log;
mod error;
mod server;

pub use crate::arq::Reliability;
pub use crate::server::*;
pub use crate::socket::*;
pub use crate::log::enable_raknet_log;

#[tokio::test]
async fn test_ping_pong(){

    let s = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = s.local_addr().unwrap().port();

    tokio::spawn(async move {
        let mut buf = [0u8 ; 1024];
        let (size , addr ) = s.recv_from(&mut buf).await.unwrap();

        let _pong = packet::read_packet_ping(&buf[..size]).await.unwrap();
        let packet = packet::PacketUnconnectedPong{
            time: utils::cur_timestamp_millis(),
            magic: true,
            guid: rand::random(),
            motd : format!("MCPE;Dedicated Server;486;1.18.11;0;10;12322747879247233720;Bedrock level;Survival;1;{};", s.local_addr().unwrap().port())
        };

        let buf = packet::write_packet_pong(&packet).await.unwrap();

        s.send_to(buf.as_slice(), addr).await.unwrap();
    });

    let addr = format!("127.0.0.1:{}", port);
    let latency = socket::RaknetSocket::ping(&addr.as_str().parse().unwrap()).await.unwrap();
    assert!((0..10).contains(&latency));
}

#[tokio::test]
async fn test_connect(){
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        assert!(client1.local_addr().unwrap() == local_addr);
        client1.send(&[1,2,3] , Reliability::Reliable).await.unwrap();
    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    assert!(client2.peer_addr().unwrap() == local_addr);
    let buf = client2.recv().await.unwrap();
    assert!(buf == vec![1,2,3]);
}

#[tokio::test]
async fn test_send_recv_fragment_data(){
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        assert!(client1.local_addr().unwrap() == local_addr);

        let mut a = vec![3u8;1000];
        let mut b = vec![2u8;1000];
        let mut c = vec![1u8;1000];
        b.append(&mut a);
        c.append(&mut b);

        client1.send(&c , Reliability::ReliableOrdered).await.unwrap();
    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    assert!(client2.peer_addr().unwrap() == local_addr);
    let buf = client2.recv().await.unwrap();
    assert!(buf.len() == 3000);
    assert!(buf[0..1000] == vec![1u8;1000]);
    assert!(buf[1000..2000] == vec![2u8;1000]);
    assert!(buf[2000..3000] == vec![3u8;1000]);
}

#[tokio::test]
async fn test_send_recv_more_reliability_type_packet(){
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        assert!(client1.local_addr().unwrap() == local_addr);

        client1.send(&[0xfe,1,2,3], Reliability::Unreliable).await.unwrap();
        let data = client1.recv().await.unwrap();
        assert!(data == [0xfe,4,5,6].to_vec());

        client1.send(&[0xfe,7,8,9], Reliability::UnreliableSequenced).await.unwrap();
        let data = client1.recv().await.unwrap();
        assert!(data == [0xfe,10,11,12].to_vec());

        client1.send(&[0xfe,13,14,15], Reliability::Reliable).await.unwrap();
        let data = client1.recv().await.unwrap();
        assert!(data == [0xfe,16,17,18].to_vec());

        let mut a = vec![3u8;1000];
        let mut b = vec![2u8;1000];
        let mut c = vec![1u8;1000];
        b.append(&mut a);
        c.append(&mut b);

        client1.send(&c , Reliability::ReliableOrdered).await.unwrap();

        let buf = client1.recv().await.unwrap();
        assert!(buf.len() == 3000);
        assert!(buf[0..1000] == vec![1u8;1000]);
        assert!(buf[1000..2000] == vec![2u8;1000]);
        assert!(buf[2000..3000] == vec![3u8;1000]);

        client1.send(&[0xfe,19,20,21], Reliability::ReliableSequenced).await.unwrap();
        let data = client1.recv().await.unwrap();
        assert!(data == [0xfe,22,23,24].to_vec());
    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    assert!(client2.peer_addr().unwrap() == local_addr);
    
    let buf = client2.recv().await.unwrap();
    assert!(buf == [0xfe,1,2,3]);

    client2.send(&[0xfe,4,5,6], Reliability::Unreliable).await.unwrap();

    let buf = client2.recv().await.unwrap();
    assert!(buf == [0xfe,7,8,9]);

    client2.send(&[0xfe,10,11,12], Reliability::UnreliableSequenced).await.unwrap();

    let buf = client2.recv().await.unwrap();
    assert!(buf == [0xfe,13,14,15]);

    client2.send(&[0xfe,16,17,18], Reliability::Reliable).await.unwrap();

    let buf = client2.recv().await.unwrap();
    assert!(buf.len() == 3000);
    assert!(buf[0..1000] == vec![1u8;1000]);
    assert!(buf[1000..2000] == vec![2u8;1000]);
    assert!(buf[2000..3000] == vec![3u8;1000]);

    let mut a = vec![3u8;1000];
    let mut b = vec![2u8;1000];
    let mut c = vec![1u8;1000];
    b.append(&mut a);
    c.append(&mut b);

    client2.send(&c , Reliability::ReliableOrdered).await.unwrap();

    let buf = client2.recv().await.unwrap();
    assert!(buf == [0xfe,19,20,21]);

    client2.send(&[0xfe,22,23,24], Reliability::ReliableSequenced).await.unwrap();
}

#[tokio::test]
async fn test_loss_packet1(){
    let notify = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify2 = notify.clone();
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        // 80% loss packet rate
        client1.set_loss_rate(8);

        for i in 0..10{
            let mut flag = vec![0xfe_u8];
            let mut data = vec![i as u8; 2000];
            flag.append(&mut data);
            client1.send(&flag, Reliability::ReliableOrdered).await.unwrap();

            let data = client1.recv().await.unwrap();
            assert!(data == flag);
        }
        notify.notify_one();

    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    // 80% loss packet rate
    client2.set_loss_rate(8);

    for i in 0..10{
        let mut flag = vec![0xfe_u8];
        let mut data = vec![i as u8; 2000];
        flag.append(&mut data);
        client2.send(&flag, Reliability::ReliableOrdered).await.unwrap();

        let data = client2.recv().await.unwrap();
        assert!(data == flag);
    }
    notify2.notified().await;
}

#[tokio::test]
async fn test_loss_packet2(){
    let notify = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify2 = notify.clone();
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        // 80% loss packet rate
        client1.set_loss_rate(8);

        for i in 0..10{
            let mut flag = vec![0xfe_u8];
            let mut data = vec![i as u8; 2000];
            flag.append(&mut data);
            client1.send(&flag, Reliability::ReliableOrdered).await.unwrap();
        }

        for i in 0..10{
            let mut flag = vec![0xfe_u8];
            let mut data = vec![i as u8; 2000];
            flag.append(&mut data);
            let data = client1.recv().await.unwrap();
            assert!(data == flag);
        }
        notify.notify_one();
    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    // 80% loss packet rate
    client2.set_loss_rate(8);

    for i in 0..10{
        let mut flag = vec![0xfe_u8];
        let mut data = vec![i as u8; 2000];
        flag.append(&mut data);
        client2.send(&flag, Reliability::ReliableOrdered).await.unwrap();
    }

    for i in 0..10{
        let mut flag = vec![0xfe_u8];
        let mut data = vec![i as u8; 2000];
        flag.append(&mut data);
        let data = client2.recv().await.unwrap();
        assert!(data == flag);
    }
    notify2.notified().await;
}

#[tokio::test]
async fn test_loss_packet_with_sequenced(){
    let notify = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify2 = notify.clone();
    let mut server = RaknetListener::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let local_addr = server.local_addr().unwrap();
    server.listen().await;
    tokio::spawn(async move {
        let mut client1 = server.accept().await.unwrap();
        // 80% loss packet rate
        client1.set_loss_rate(8);

        for i in 0..100{
            let mut flag = vec![0xfe_u8];
            let mut data = vec![i as u8; 20];
            flag.append(&mut data);
            client1.send(&flag, Reliability::ReliableSequenced).await.unwrap();
        }

        let mut last = 0;
        for i in 0..50{
            let mut flag = vec![0xfe_u8];
            let mut data = vec![i as u8; 20];
            flag.append(&mut data);
            let data = client1.recv().await.unwrap();
            assert!(data[1] >= last);
            last = data[1];
        }
        notify.notify_one();
    });
    let mut client2 = RaknetSocket::connect(&local_addr).await.unwrap();
    // 80% loss packet rate
    client2.set_loss_rate(8);

    for i in 0..100{
        let mut flag = vec![0xfe_u8];
        let mut data = vec![i as u8; 20];
        flag.append(&mut data);
        client2.send(&flag, Reliability::ReliableSequenced).await.unwrap();
    }

    let mut last = 0;
    for i in 0..50{
        let mut flag = vec![0xfe_u8];
        let mut data = vec![i as u8; 20];
        flag.append(&mut data);
        let data = client2.recv().await.unwrap();
        assert!(data[1] >= last);
        last = data[1];

    }
    notify2.notified().await;
}

/*
#[tokio::test]
async fn chore(){
    let mut client = RaknetSocket::connect(&"192.168.199.127:19132".parse().unwrap()).await.unwrap();
    let mut a = vec![3u8;1000];
    let mut b = vec![2u8;1000];
    let mut c = vec![0xfe;1000];
    b.append(&mut a);
    c.append(&mut b);
    client.send(&c, Reliability::ReliableOrdered).await.unwrap();
    client.recv().await.unwrap();
}

#[tokio::test]
async fn chore2(){

    enbale_raknet_log(true);
    let mut listener = RaknetListener::bind("0.0.0.0:19199".parse().unwrap()).await.unwrap();
    listener.listen().await;
    loop{
        let mut client1 = listener.accept().await.unwrap();
        let mut client2 = RaknetSocket::connect(&"192.168.199.127:19132".parse().unwrap()).await.unwrap();
        tokio::spawn(async move {
            println!("build connection");
            loop{
                tokio::select!{
                    a = client1.recv() => {
                        let a = match a{
                            Ok(p) => p,
                            Err(_) => {
                                client2.close().await.unwrap();
                                break;
                            },
                        };
                        match client2.send(&a, Reliability::ReliableOrdered).await{
                            Ok(p) => p,
                            Err(_) => {
                                client1.close().await.unwrap();
                                break;
                            },
                        };
                    },
                    b = client2.recv() => {
                        let b = match b{
                            Ok(p) => p,
                            Err(_) => {
                                client1.close().await.unwrap();
                                break;
                            },
                        };
                        match client1.send(&b, Reliability::ReliableOrdered).await{
                            Ok(p) => p,
                            Err(_) => {
                                client2.close().await.unwrap();
                                break;
                            },
                        };
                    }
                }
            }
            println!("close connection");
        });
    }


}
*/