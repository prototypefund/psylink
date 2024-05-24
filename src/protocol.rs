// Keep these in sync with arduino code.
pub const DELAY_PARAM_A: f64 = -11.3384217;
pub const DELAY_PARAM_B: f64 = 1.93093431;
pub const SAMPLE_VALUE_OFFSET: i32 = -127;

pub struct Decoder {
    last_tick: Option<u8>,
    channel_count: u32,
}

pub struct Packet {
    channel_count: u32,
    tick: u32,
    min_sampling_delay: f64,
    max_sampling_delay: f64,
    sample_count: u32,
    samples: Vec<Vec<u8>>,
    is_duplicate: bool,
    lost_packets: u32,
}

impl Decoder {
    fn new(channel_count: u32) -> Decoder {
        Self {
            last_tick: None,
            channel_count,
        }
    }

    fn decode_packet(&mut self, packet: Vec<u8>) -> Packet {
        return Packet {
            channel_count: self.channel_count,
            tick: 0,                       // TODO
            min_sampling_delay: 0.0,       // TODO
            max_sampling_delay: 0.0,       // TODO
            sample_count: 1,               // TODO
            samples: vec![packet.clone()], // TODO
            is_duplicate: false,           // TODO
            lost_packets: 0,               // TODO
        };
    }
}

#[test]
fn test_decoding() {
    let channel_count = 8;
    let mut decoder = Decoder::new(channel_count);
    let packet: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7];

    let packet = decoder.decode_packet(packet);
    assert_eq!(packet.channel_count, channel_count);
}
