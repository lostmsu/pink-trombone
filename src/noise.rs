use std::f64::consts::PI;

pub trait NoiseSource<T> {
    fn noise(&mut self) -> T;
}

pub fn new_filtered_noise_source(
    f0: f64,
    q: f64,
    sample_rate: u32,
    loop_size: usize,
    rng: &mut dyn NoiseSource<f64>,
) -> Box<dyn FnMut() -> f64 + Send + 'static> {
    let mut white_noise = new_looped_white_noise(loop_size, rng);
    let mut filter = new_bandpass_filter(f0, q, sample_rate);
    Box::new(move || filter.filter(white_noise.noise()))
}

fn new_looped_white_noise(
    loop_size: usize,
    rng: &mut dyn NoiseSource<f64>,
) -> impl NoiseSource<f64> {
    let noise = (0..loop_size).map(|_| 2.0 * rng.noise() - 1.0).collect();
    LoopedNoiseBuffer {
        noise,
        current_index: 0,
    }
}

struct LoopedNoiseBuffer {
    noise: Vec<f64>,
    current_index: usize,
}

impl NoiseSource<f64> for LoopedNoiseBuffer {
    fn noise(&mut self) -> f64 {
        if self.current_index >= self.noise.len() {
            self.current_index = 0
        }
        let val = self.noise[self.current_index];
        self.current_index += 1;
        val
    }
}

pub trait Filter {
    fn filter(&mut self, x: f64) -> f64;
}

pub fn new_bandpass_filter(f0: f64, q: f64, sample_rate: u32) -> impl Filter {
    let w0 = 2.0 * PI * f0 / sample_rate as f64;
    let alpha = w0.sin() / (2.0 * q);
    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha;
    BiquadIirFilter::new(b0, b1, b2, a0, a1, a2)
}

struct BiquadIirFilter {
    nb0: f64,
    nb1: f64,
    nb2: f64,
    na1: f64,
    na2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadIirFilter {
    pub fn new(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> BiquadIirFilter {
        BiquadIirFilter {
            nb0: b0 / a0,
            nb1: b1 / a0,
            nb2: b2 / a0,
            na1: a1 / a0,
            na2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl Filter for BiquadIirFilter {
    fn filter(&mut self, x: f64) -> f64 {
        let y = self.nb0 * x + self.nb1 * self.x1 + self.nb2 * self.x2
            - self.na1 * self.y1
            - self.na2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}
