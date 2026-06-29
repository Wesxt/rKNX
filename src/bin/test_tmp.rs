use rknx::connection::server::{KnxNetIpServer, KnxNetIpServerOptions};
use rknx::core::knxnetip_enum::KnxNetIpServiceType;
use rknx::core::knxnetip_header::KnxNetIpHeader;
use tokio::net::UdpSocket;
use rknx::connection::KnxService;

#[tokio::main]
async fn main() {
    let test_port = 54321u16;
    let options_fixed = KnxNetIpServerOptions {
        ip: "224.0.23.12".to_string(),
        port: test_port,
        local_ip: "127.0.0.1".to_string(),
        individual_address: "1.1.2".to_string(),
        friendly_name: "test".to_string(),
        mac_address: "00:00:00:00:00:00".to_string(),
        routing_delay: 20,
        client_addrs: None,
        serial_number: None,
        use_all_interfaces: false,
        is_routing: false,
        max_pending_requests_per_client: 100,
    };
    let router = KnxNetIpServer::new(options_fixed);
    let mut sub = router.subscribe();
    router.connect().await.unwrap();

    let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let cemi_bytes: Vec<u8> = vec![
        0x29, 0x00, 0xBC, 0xE0, 0x11, 0x01, 0x09, 0x02, 0x01, 0x00, 0x81,
    ];

    let header = KnxNetIpHeader::new(
        KnxNetIpServiceType::RoutingIndication,
        (6 + cemi_bytes.len()) as u16,
    );
    let mut packet = header.to_buffer();
    packet.extend_from_slice(&cemi_bytes);

    println!("Sending packet...");
    sender.send_to(&packet, format!("127.0.0.1:{}", test_port)).await.unwrap();

    println!("Waiting for receive...");
    match tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv()).await {
        Ok(res) => println!("Received: {:?}", res),
        Err(e) => println!("Timed out: {:?}", e),
    }
}
