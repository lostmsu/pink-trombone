use std::f32::consts::PI;

use crate::{
    math::interpolate,
    noise::{self, NoiseSource},
    noise_gen::NoiseGenerator,
};

pub struct Glottis {
    pub always_voice: bool,
    pub auto_wobble: bool,
    is_touched: bool,
    pub target_tenseness: f32,
    pub target_frequency: f32,
    pub vibrato_amount: f32,
    pub vibrato_frequency: f32,

    noise_generator: NoiseGenerator,

    sample_rate: u32,
    sample_count: u64,
    pub intensity: f32,
    pub loudness: f32,
    smooth_frequency: f32,
    time_in_waveform: f32,
    old_tenseness: f32,
    new_tenseness: f32,
    old_frequency: f32,
    new_frequency: f32,
    aspiration_noise_source: Box<dyn FnMut() -> f64 + Send + 'static>,
    waveform_length: f32,

    // waveform state
    alpha: f32,
    e0: f32,
    epsilon: f32,
    shift: f32,
    delta: f32,
    te: f32,
    omega: f32,
}

impl Glottis {
    pub fn new(sample_rate: u32, rng: &mut dyn NoiseSource<f64>, seed: u16) -> Glottis {
        let mut glottis = Glottis {
            always_voice: true,
            auto_wobble: true,
            is_touched: false,
            target_tenseness: 0.6,
            target_frequency: 140.0,
            vibrato_amount: 0.005,
            vibrato_frequency: 6.0,

            noise_generator: NoiseGenerator::new(seed),

            sample_rate,

            sample_count: 0,
            intensity: 0.0,
            loudness: 1.0,
            smooth_frequency: 140.0,
            time_in_waveform: 0.0,
            old_tenseness: 0.6,
            new_tenseness: 0.6,
            old_frequency: 140.0,
            new_frequency: 140.0,

            aspiration_noise_source: noise::new_filtered_noise_source(
                500.0,
                0.5,
                sample_rate,
                0x8000,
                rng,
            ),

            waveform_length: 0.0,

            // waveform state
            alpha: 0.0,
            e0: 0.0,
            epsilon: 0.0,
            shift: 0.0,
            delta: 0.0,
            te: 0.0,
            omega: 0.0,
        };

        glottis.setup_waveform(0.0);

        glottis
    }

    pub fn set_musical_note(&mut self, semitone: f32) {
        const A4: f32 = 440.0;
        self.target_frequency = A4 * 2.0_f32.powf(semitone * (1.0 / 12.0));
    }

    pub fn step(&mut self, lambda: f32) -> f32 {
        let time = self.sample_count as f32 / self.sample_rate as f32;

        if self.time_in_waveform > self.waveform_length {
            self.time_in_waveform -= self.waveform_length;
            self.setup_waveform(lambda);
        }

        let out1 = self.normalized_lf_waveform(self.time_in_waveform / self.waveform_length);
        let asp_noise = (self.aspiration_noise_source)() as f32;
        let aspiration1 = self.intensity
            * (1.0 - self.target_tenseness.sqrt())
            * self.get_noise_modulator()
            * asp_noise;
        let aspiration2 = aspiration1 * (0.2 + 0.02 * self.noise_generator.simplex(time * 1.99));
        let result = out1 + aspiration2;
        self.sample_count += 1;
        self.time_in_waveform += 1.0 / self.sample_rate as f32;
        result
    }

    pub fn get_noise_modulator(&self) -> f32 {
        let voiced =
            0.1 + 0.2 * 0_f32.max((PI * 2.0 * self.time_in_waveform / self.waveform_length).sin());
        self.target_tenseness * self.intensity * voiced
            + (1.0 - self.target_tenseness * self.intensity) * 0.3
    }

    pub fn adjust_parameters(&mut self, delta_time: f32) {
        let delta = delta_time * self.sample_rate as f32 / 512.0;
        let old_time = self.sample_count as f32 / self.sample_rate as f32;
        let new_time = old_time + delta_time;
        self.adjust_intensity(delta);
        self.calculate_new_frequency(new_time, delta);
        self.calculate_new_tenseness(new_time);
    }

    fn calculate_new_frequency(&mut self, time: f32, delta_time: f32) {
        if self.intensity == 0.0 {
            self.smooth_frequency = self.target_frequency;
        } else if self.target_frequency > self.smooth_frequency {
            self.smooth_frequency = self
                .target_frequency
                .min(self.smooth_frequency * (1.0 + 0.1 * delta_time));
        } else if self.target_frequency < self.smooth_frequency {
            self.smooth_frequency = self
                .target_frequency
                .max(self.smooth_frequency / (1.0 + 0.1 * delta_time));
        }

        self.old_frequency = self.new_frequency;
        self.new_frequency =
            (self.smooth_frequency * (1.0 + self.calculate_vibrato(time))).max(10.0);
    }

    fn calculate_new_tenseness(&mut self, time: f32) {
        self.old_tenseness = self.new_tenseness;
        self.new_tenseness = self.target_tenseness
            + 0.1 * self.noise_generator.simplex(time * 0.46)
            + 0.05 * self.noise_generator.simplex(time * 0.36);
        self.new_tenseness = self.new_tenseness.max(0.0);

        if !self.is_touched && self.always_voice {
            // attack
            self.new_tenseness += (3.0 - self.target_tenseness) * (1.0 - self.intensity);
        }
    }

    fn adjust_intensity(&mut self, delta: f32) {
        if self.is_touched || self.always_voice {
            self.intensity += 0.13 * delta;
        } else {
            self.intensity -= 0.05 * delta;
        }

        self.intensity = self.intensity.clamp(0.0, 1.0);
    }

    fn calculate_vibrato(&mut self, time: f32) -> f32 {
        let mut vibrato = self.vibrato_amount * (PI * 2.0 * time * self.vibrato_frequency).sin();
        vibrato += 0.02 * self.noise_generator.simplex(time * 4.07);
        vibrato += 0.04 * self.noise_generator.simplex(time * 2.15);
        if self.auto_wobble {
            vibrato += 0.2 * self.noise_generator.simplex(time * 0.96);
            vibrato += 0.4 * self.noise_generator.simplex(time * 0.5);
        }
        vibrato
    }

    fn setup_waveform(&mut self, lambda: f32) {
        let frequency = interpolate(self.old_frequency, self.new_frequency, lambda);
        let tenseness = interpolate(self.old_tenseness, self.new_tenseness, lambda);
        self.waveform_length = 1.0 / frequency;
        self.loudness = tenseness.max(0.0).powf(0.25);

        let rd = (3.0 * (1.0 - tenseness)).clamp(0.5, 2.7);

        // normalized to time = 1, Ee = 1
        let ra = -0.01 + 0.048 * rd;

        let rk = 0.224 + 0.118 * rd;
        let rg = (rk / 4.0) * (0.5 + 1.2 * rk) / (0.11 * rd - ra * (0.5 + 1.2 * rk));

        let ta = ra;
        let tp = 1.0 / (2.0 * rg);
        let te = tp + tp * rk;

        let epsilon = 1.0 / ta;
        let shift = (-epsilon * (1.0 - te)).exp();
        let delta = 1.0 - shift; // divide by self to scale RHS

        let rhs_integral = ((1.0 / epsilon) * (shift - 1.0) + (1.0 - te) * shift) / delta;
        let total_lower_integral = rhs_integral - (te - tp) / 2.0;
        let total_upper_integral = -total_lower_integral;

        let omega = PI / tp;
        let s = (omega * te).sin();

        // need E0*e^(alpha*Te)*s = -1 (to meet the return at -1)
        // and E0*e^(alpha*Tp/2) * Tp*2/pi = totalUpperIntegral
        //             (our approximation of the integral up to Tp)
        // writing x for e^alpha,
        // have E0*x^Te*s = -1 and E0 * x^(Tp/2) * Tp*2/pi = totalUpperIntegral
        // dividing the second by the first,
        // letting y = x^(Tp/2 - Te),
        // y * Tp*2 / (pi*s) = -totalUpperIntegral;

        let y = -PI * s * total_upper_integral / (tp * 2.0);
        let z = y.ln();
        let alpha = z / (tp / 2.0 - te);
        let e0 = -1.0 / (s * (alpha * te).exp());

        self.alpha = alpha;
        self.e0 = e0;
        self.epsilon = epsilon;
        self.shift = shift;
        self.delta = delta;
        self.te = te;
        self.omega = omega;
    }

    fn normalized_lf_waveform(&self, t: f32) -> f32 {
        let output = if t > self.te {
            (-(-self.epsilon * (t - self.te)).exp() + self.shift) / self.delta
        } else {
            self.e0 * (self.alpha * t).exp() * (self.omega * t).sin()
        };
        output * self.intensity * self.loudness
    }
}
