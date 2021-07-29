use std::sync::{Arc, Mutex};

use pink_trombone::{NoiseSource, PinkTrombone};
use rand::Rng;
use rodio::{OutputStream, Source};

struct ThreadRng {}

impl NoiseSource<f64> for ThreadRng {
    fn noise(&mut self) -> f64 {
        let mut rng = rand::thread_rng();
        rng.gen()
    }
}

#[derive(Clone)]
struct PinkTromboneSource {
    trombone: Arc<Mutex<PinkTrombone>>,
    buffer_pos: usize,
    buffer: [f32; 512],
}

impl PinkTromboneSource {
    pub fn new(trombone: PinkTrombone) -> PinkTromboneSource {
        let buffer = [0_f32; 512];
        PinkTromboneSource {
            trombone: Arc::new(Mutex::new(trombone)),
            buffer_pos: buffer.len(),
            buffer,
        }
    }
}

impl Iterator for PinkTromboneSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.buffer_pos == self.buffer.len() {
            self.trombone.lock().unwrap().synthesize(&mut self.buffer);
            self.buffer_pos = 0;
        }
        let result = self.buffer[self.buffer_pos];
        assert!(result.abs() <= 1.0);
        self.buffer_pos += 1;
        Some(result)
    }
}

impl Source for PinkTromboneSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(512)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.trombone.lock().unwrap().sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

const SAMPLE_RATE: u32 = 48000;

fn main() {
    let mut random = ThreadRng {};
    let seed = rand::thread_rng().gen();
    let trombone = PinkTrombone::new(SAMPLE_RATE, &mut random, seed);
    let source = PinkTromboneSource::new(trombone);

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    stream_handle.play_raw(source.clone()).unwrap();

    for tone in 0..24 {
        {
            let mut src = source.trombone.lock().unwrap();
            src.set_musical_note(tone as f32);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    for tone in (0..23).rev() {
        {
            let mut src = source.trombone.lock().unwrap();
            src.set_musical_note(tone as f32);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}
