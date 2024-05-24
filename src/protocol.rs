// Keep these in sync with arduino code.
pub const SAMPLE_DELAY_PARAM_A: f64 = -11.3384217;
pub const SAMPLE_DELAY_PARAM_B: f64 = 1.93093431;
pub const SAMPLE_VALUE_OFFSET: i32 = -127;

pub struct Decoder {
    last_tick: Option<u32>,
    channel_count: u32,
}

pub struct Packet {
    pub channel_count: u32,
    pub tick: u32,
    pub min_sampling_delay: f64,
    pub max_sampling_delay: f64,
    pub sample_count: u32,
    pub samples: Vec<Vec<u8>>,
    pub is_duplicate: bool,
    pub lost_packets: u32,
}

impl Decoder {
    fn new(channel_count: u32) -> Decoder {
        Self {
            last_tick: None,
            channel_count,
        }
    }

    fn decode_packet(&mut self, packet: Vec<u8>) -> Result<Packet, String> {
        let tick: u32 = *packet
            .get(0)
            .ok_or("Failed to decode packet, no Tick supplied")? as u32;

        let delay_byte: u8 = *packet
            .get(1)
            .ok_or("Failed to decode packet, no sampling delay byte supplied")?;

        let (min_sampling_delay, max_sampling_delay) = decompress_delay(delay_byte);

        let is_duplicate = if let Some(last_tick) = self.last_tick {
            tick == last_tick
        } else {
            false
        };

        let lost_packets: u32 = if let Some(last_tick) = self.last_tick {
            if tick > last_tick { tick } else { tick + 255 }
                .saturating_sub(last_tick)
                .saturating_sub(1)
        } else {
            0
        };

        self.last_tick = Some(tick);

        return Ok(Packet {
            channel_count: self.channel_count,
            tick,
            min_sampling_delay,
            max_sampling_delay,
            sample_count: 1,               // TODO
            samples: vec![packet.clone()], // TODO
            is_duplicate,
            lost_packets,
        });
    }
}

/// PsyLink transmits the information about its sampling interval delay in a single byte, we have
/// to decode it to make use of it.  We will get an approximate value for the minimum delay between
/// two samplings, and the maximum one.
fn decompress_delay(delay_byte: u8) -> (f64, f64) {
    let min_delay = (delay_byte & 0xf0) >> 4;
    let max_delay = delay_byte & 0x0f;
    return (
        decompress_delay_4bit(min_delay),
        decompress_delay_4bit(max_delay),
    );
}

#[inline]
fn decompress_delay_4bit(delay_4bit: u8) -> f64 {
    ((delay_4bit as f64 - SAMPLE_DELAY_PARAM_A) / SAMPLE_DELAY_PARAM_B).exp()
}

#[test]
fn test_decoding() {
    let channel_count = 8;
    let mut decoder = Decoder::new(channel_count);
    let packet_data_1: Vec<u8> = vec![
        45, 21, 127, 124, 126, 175, 122, 239, 122, 6, 139, 110, 128, 131, 94, 116, 123, 205, 159,
        103, 128, 136, 90, 133, 120, 203, 144, 104, 85, 136, 86, 133, 121, 6, 143, 130, 130, 139,
        94, 146, 122, 205, 138, 130, 128, 137, 95, 132, 124, 205, 144, 138, 127, 139, 94, 138, 122,
        6, 144, 108, 86, 133, 87, 108, 121, 17, 145, 103, 85, 137, 88, 119, 123, 205, 158, 119,
        129, 131, 95, 119, 121, 15, 143, 112, 84, 134, 87, 124, 122, 6, 143, 114, 86, 132, 90, 120,
        124, 205, 160, 107, 126, 138, 92, 148, 121, 205, 147, 100, 87, 136, 90, 134, 121, 16, 146,
        112, 83, 133, 88, 124, 121, 205, 146, 103, 93, 135, 94, 133, 121, 17, 145, 104, 125, 135,
        93, 131, 122, 42, 143, 109, 81, 137, 90, 143, 123, 205, 157, 124, 125, 139, 91, 156, 122,
        205, 147, 101, 86, 137, 87, 132, 124, 205, 153, 129, 126, 139, 94, 145, 122, 205, 146, 101,
        83, 137, 88, 133, 121, 205, 148, 100, 90, 136, 89, 133, 121, 22, 144, 128, 128, 138, 95,
        143, 122, 205, 159, 115, 126, 138, 94, 147, 120, 205, 147, 102, 82, 136, 88, 133,
    ];
    let packet_data_2: Vec<u8> = vec![
        47, 21, 127, 124, 126, 174, 129, 240, 122, 27, 139, 116, 82, 134, 103, 127, 123, 205, 140,
        106, 86, 136, 103, 129, 122, 205, 142, 108, 86, 137, 104, 127, 122, 205, 142, 108, 86, 135,
        106, 127, 122, 205, 145, 106, 87, 135, 106, 127, 123, 205, 155, 118, 125, 140, 103, 128,
        123, 205, 154, 120, 124, 140, 103, 129, 123, 205, 157, 111, 124, 140, 103, 128, 124, 205,
        138, 131, 124, 137, 102, 128, 124, 205, 154, 120, 124, 140, 102, 129, 124, 205, 151, 120,
        123, 140, 101, 129, 121, 205, 140, 121, 124, 139, 99, 130, 123, 205, 142, 108, 82, 136,
        105, 127, 121, 12, 139, 120, 126, 133, 103, 128, 122, 205, 144, 109, 83, 135, 105, 127,
        122, 205, 151, 106, 102, 135, 104, 127, 124, 205, 152, 106, 100, 134, 104, 127, 121, 184,
        139, 130, 125, 137, 100, 130, 122, 205, 138, 123, 124, 138, 100, 129, 122, 12, 138, 131,
        125, 131, 104, 125, 123, 205, 155, 107, 124, 135, 105, 126, 124, 205, 153, 106, 124, 135,
        104, 126, 122, 191, 140, 122, 124, 137, 101, 129, 122, 12, 139, 132, 124, 136, 101, 130,
        124, 205, 153, 106, 125, 136, 103, 127,
    ];

    let packet = decoder.decode_packet(packet_data_1);
    assert!(packet.is_ok());
    let packet = packet.unwrap();

    assert_eq!(packet.channel_count, channel_count);
    assert_eq!(packet.tick, 45);
    assert_eq!(packet.is_duplicate, false);
    approx_eq::assert_approx_eq!(packet.min_sampling_delay, 595.779, 1e-3);
    approx_eq::assert_approx_eq!(packet.max_sampling_delay, 4728.708, 1e-3);
    assert_eq!(packet.lost_packets, 0);

    let packet = decoder.decode_packet(packet_data_2);
    assert!(packet.is_ok());
    let packet = packet.unwrap();
    assert_eq!(packet.tick, 47);
    assert_eq!(packet.lost_packets, 1); // packet 46 was missing
}
