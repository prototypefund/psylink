use rodio::{source::SineWave, OutputStream, Sink, Source};
use std::time::Duration;

pub fn play(frequency: f32) {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let source = SineWave::new(frequency)
        .take_duration(Duration::from_secs_f32(1.0))
        .amplify(0.20);
    sink.append(source);
    sink.sleep_until_end();
}
