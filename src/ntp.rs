use embassy_net::dns::DnsQueryType;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use sntpc::NtpContext;
use sntpc::NtpTimestampGenerator;

use crate::NTP_SERVER;

#[derive(Copy, Clone)]
pub struct Timestamp {
    init_time: embassy_time::Instant,
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp {
            init_time: embassy_time::Instant::now(),
        }
    }
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        self.init_time = embassy_time::Instant::now();
    }

    fn timestamp_sec(&self) -> u64 {
        0
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        0
    }
}

pub struct NtpClient {
    addr: core::net::SocketAddr,
    context: NtpContext<Timestamp>,
}

pub struct NtpStackResources {
    pub rx_meta: [PacketMetadata; 16],
    pub rx_buffer: [u8; 4096],
    pub tx_meta: [PacketMetadata; 16],
    pub tx_buffer: [u8; 4096],
}

impl Default for NtpStackResources {
    fn default() -> Self {
        Self {
            rx_meta: [PacketMetadata::EMPTY; 16],
            rx_buffer: [0; 4096],
            tx_meta: [PacketMetadata::EMPTY; 16],
            tx_buffer: [0; 4096],
        }
    }
}

impl NtpClient {
    pub async fn new<'network>(
        network_stack: embassy_net::Stack<'network>,
        ntp_stack_resources: &'network mut NtpStackResources,
    ) -> Result<(UdpSocket<'network>, crate::ntp::NtpClient), NtpClientError> {
        // Create UDP socket

        let mut udp_socket = UdpSocket::new(
            network_stack,
            &mut ntp_stack_resources.rx_meta,
            &mut ntp_stack_resources.rx_buffer,
            &mut ntp_stack_resources.tx_meta,
            &mut ntp_stack_resources.tx_buffer,
        );
        udp_socket.bind(123).unwrap();

        let addrs = network_stack
            .dns_query(NTP_SERVER, DnsQueryType::A)
            .await
            .map_err(|error| {
                defmt::error!("Failed to run DNS query for {}: {:?}", NTP_SERVER, error);
                NtpClientError::RunDns(error)
            })?;

        if addrs.is_empty() {
            defmt::error!("Failed to resolve DNS {}", NTP_SERVER);
            return Err(NtpClientError::ResolveDns);
        }

        let context = NtpContext::new(crate::ntp::Timestamp::default());
        Ok((
            udp_socket,
            Self {
                addr: core::net::SocketAddr::from((addrs[0], 123)),
                context,
            },
        ))
    }

    pub async fn get_time<U>(&self, udp_socket: &U) -> Result<sntpc::NtpResult, NtpClientError>
    where
        U: sntpc::NtpUdpSocket,
    {
        sntpc::get_time(self.addr, udp_socket, self.context)
            .await
            .map_err(NtpClientError::GetTimeFailed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NtpClientError {
    #[error("Failed to get time: {:?}", .0)]
    GetTimeFailed(sntpc::Error),

    #[error("Failed to run DNS query for NTP server '{}': {:?}", NTP_SERVER, .0)]
    RunDns(embassy_net::dns::Error),

    #[error("Failed to resolve DNS for NTP server '{}'", NTP_SERVER)]
    ResolveDns,
}

impl defmt::Format for NtpClientError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "Error: {:?}", self)
    }
}
