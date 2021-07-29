use crate::glottis::Glottis;
use crate::math::{interpolate, sqr};
use crate::noise::{self, NoiseSource};
use crate::transient::Transient;
use crate::turbulence::TurbulencePoint;

pub struct Tract {
    pub glottis: Glottis,
    sample_rate: u32,
    frication_noise_source: Box<dyn FnMut() -> f64 + Send + 'static>,

    sample_count: usize,
    pub time: f32,

    left: [f64; Tract::N],
    right: [f64; Tract::N],
    reflection: [f64; Tract::N],
    new_reflection: [f64; Tract::N],
    junction_output_right: [f64; Tract::N],
    justion_output_left: [f64; Tract::N + 1],
    max_amplitude: [f64; Tract::N],
    /// vocal tract cell diameters
    pub diameter: [f64; Tract::N],

    pub transients: Vec<Transient>,
    pub turbulence_points: Vec<TurbulencePoint>,

    nose_right: [f64; NOSE_LEN],
    nose_left: [f64; NOSE_LEN],
    nose_junction_output_right: [f64; NOSE_LEN],
    nose_junction_output_left: [f64; NOSE_LEN + 1],
    nose_reflection: [f64; NOSE_LEN],
    pub nose_diameter: [f64; NOSE_LEN],
    nose_max_amplitude: [f64; NOSE_LEN],

    reflection_left: f64,
    reflection_right: f64,
    new_reflection_left: f64,
    new_reflection_right: f64,
    reflection_nose: f64,
    new_reflection_nose: f64,
}

const N: usize = Tract::N;

const GLOTTAL_REFLECTION: f64 = 0.75;
const LIP_REFLECTION: f64 = -0.85;

const NOSE_LEN: usize = 28;
const NOSE_START: usize = N - NOSE_LEN + 1;

impl Tract {
    pub const N: usize = 44;
    pub const BLADE_START: usize = 10;
    pub const TIP_START: usize = 32;
    pub const LIP_START: usize = 39;
    pub const NOSE_START: usize = NOSE_START;
    pub const NOSE_LEN: usize = NOSE_LEN;

    pub fn new(glottis: Glottis, sample_rate: u32, rng: &mut dyn NoiseSource<f64>) -> Tract {
        if sample_rate == 0 {
            panic!("sample_rate must be > 0")
        };

        Tract {
            glottis,
            sample_rate,
            frication_noise_source: noise::new_filtered_noise_source(
                1000.0,
                0.5,
                sample_rate,
                0x8000,
                rng,
            ),

            transients: Vec::new(),
            turbulence_points: Vec::new(),

            sample_count: 0,
            time: 0.0,
            left: [0.0; Tract::N],
            right: [0.0; Tract::N],
            reflection: [0.0; Tract::N],
            new_reflection: [0.0; Tract::N],
            junction_output_right: [0.0; Tract::N],
            justion_output_left: [0.0; Tract::N + 1],
            max_amplitude: [0.0; Tract::N],
            diameter: [0.0; Tract::N],

            nose_right: [0.0; NOSE_LEN],
            nose_left: [0.0; NOSE_LEN],
            nose_junction_output_right: [0.0; NOSE_LEN],
            nose_junction_output_left: [0.0; NOSE_LEN + 1],
            nose_reflection: [0.0; NOSE_LEN],
            nose_diameter: [0.0; NOSE_LEN],
            nose_max_amplitude: [0.0; NOSE_LEN],
            reflection_left: 0.0,
            reflection_right: 0.0,
            new_reflection_left: 0.0,
            new_reflection_right: 0.0,
            reflection_nose: 0.0,
            new_reflection_nose: 0.0,
        }
    }

    pub fn calculate_nose_reflections(&mut self) {
        let mut a = [0.0; NOSE_LEN];
        for i in 0..NOSE_LEN {
            a[i] = 1e-6_f64.max(sqr(self.nose_diameter[i]));
        }
        for i in 1..NOSE_LEN {
            self.nose_reflection[i] = assert_volume((a[i - 1] - a[i]) / (a[i - 1] + a[i]));
        }
    }

    pub fn calculate_new_block_parameters(&mut self) {
        self.calculate_main_tract_reflections();
        self.calculate_nose_junction_reflections();
    }

    fn calculate_main_tract_reflections(&mut self) {
        let mut a = [0.0; Tract::N];
        for i in 0..Tract::N {
            a[i] = sqr(self.diameter[i]);
        }
        for i in 1..Tract::N {
            self.reflection[i] = self.new_reflection[i];
            let sum = a[i - 1] + a[i];
            self.new_reflection[i] = if sum.abs() > 1e-6 {
                (a[i - 1] - a[i]) / sum
            } else {
                1.0
            };
        }
    }

    fn calculate_nose_junction_reflections(&mut self) {
        self.reflection_left = self.new_reflection_left;
        self.reflection_right = self.new_reflection_right;
        self.reflection_nose = self.new_reflection_nose;

        let velum_a = sqr(self.nose_diameter[0]);
        let an0 = sqr(self.diameter[NOSE_START]);
        let an1 = sqr(self.diameter[NOSE_START + 1]);
        let sum = an0 + an1 + velum_a;

        if sum.abs() > 1e-6 {
            self.new_reflection_left = (2.0 * an0 - sum) / sum;
            self.new_reflection_right = (2.0 * an1 - sum) / sum;
            self.new_reflection_nose = (2.0 * velum_a - sum) / sum;
        } else {
            self.new_reflection_left = 1.0;
            self.new_reflection_right = 1.0;
            self.new_reflection_nose = 1.0;
        }
    }

    pub fn step(&mut self, glottal_output: f64, lambda: f64) -> f32 {
        // mouth
        self.process_transients();
        self.add_turbulence_noise();

        // self.glottalReflection = -0.8 + 1.6 * self.glottis.newTenseness;
        self.junction_output_right[0] = self.left[0] * GLOTTAL_REFLECTION + glottal_output;
        self.justion_output_left[N] = self.right[N - 1] * LIP_REFLECTION;

        for i in 1..N {
            let r = interpolate(self.reflection[i], self.new_reflection[i], lambda);
            let w = r * (self.right[i - 1] + self.left[i]);
            self.junction_output_right[i] = assert_volume(self.right[i - 1] - w);
            self.justion_output_left[i] = assert_volume(self.left[i] + w);
        }

        // now at junction with nose
        let i = NOSE_START;
        let r = interpolate(self.reflection_left, self.new_reflection_left, lambda);
        self.justion_output_left[i] =
            assert_volume(r * self.right[i - 1] + (1.0 + r) * (self.nose_left[0] + self.left[i]));
        let r = interpolate(self.reflection_right, self.new_reflection_right, lambda);
        self.junction_output_right[i] =
            assert_volume(r * self.left[i] + (1.0 + r) * (self.right[i - 1] + self.nose_left[0]));
        let r = interpolate(self.reflection_nose, self.new_reflection_nose, lambda);
        self.nose_junction_output_right[0] =
            assert_volume(r * self.nose_left[0] + (1.0 + r) * (self.left[i] + self.right[i - 1]));

        for i in 0..N {
            let right = self.junction_output_right[i] * 0.999;
            let left = self.justion_output_left[i + 1] * 0.999;

            self.right[i] = right;
            self.left[i] = left;

            let amplitude = (right + left).abs();

            self.max_amplitude[i] *= 0.9999;
            self.max_amplitude[i] = self.max_amplitude[i].max(amplitude);
        }

        let lip_output = self.right[N - 1];

        // nose
        self.nose_junction_output_left[NOSE_LEN] = self.nose_right[NOSE_LEN - 1] * LIP_REFLECTION;

        for i in 1..NOSE_LEN {
            let w = self.nose_reflection[i] * (self.nose_right[i - 1] + self.nose_left[i]);
            self.nose_junction_output_right[i] = assert_volume(self.nose_right[i - 1] - w);
            self.nose_junction_output_left[i] = assert_volume(self.nose_left[i] + w);
        }

        for i in 0..NOSE_LEN {
            let right = self.nose_junction_output_right[i];
            let left = self.nose_junction_output_left[i + 1];
            self.nose_right[i] = right;
            self.nose_left[i] = left;
            let amplitude = (right + left).abs();
            self.nose_max_amplitude[i] *= 0.9999;
            self.nose_max_amplitude[i] = self.nose_max_amplitude[i].max(amplitude);
        }

        let nose_output = self.nose_right[NOSE_LEN - 1];

        self.sample_count += 1;
        self.time = self.sample_count as f32 / self.sample_rate as f32;

        (lip_output + nose_output) as f32
    }

    fn process_transients(&mut self) {
        for i in (0..self.transients.len()).rev() {
            let trans = &self.transients[i];

            let time_alive = self.time - trans.start_time;
            if time_alive > trans.life_time {
                self.transients.remove(i);
                continue;
            }
            let amplitude = trans.strength * 2.0_f64.powf(-trans.exponent * time_alive as f64);

            self.right[trans.position] += amplitude * 0.5;
            self.left[trans.position] += amplitude * 0.5;
        }
    }

    fn add_turbulence_noise(&mut self) {
        const FRICATIVE_ATTACK_TIME: f32 = 0.1; // seconds

        let mut turbulence_noises = Vec::<(f64, f64, f64)>::new();

        for p in &self.turbulence_points {
            if p.position < 2.0 || p.position > N as f32 {
                continue;
            }
            if p.diameter <= 0.0 {
                continue;
            }

            let intensity = if f32::is_nan(p.end_time) {
                (self.time - p.start_time) / FRICATIVE_ATTACK_TIME
            } else {
                1.0 - (self.time - p.end_time) / FRICATIVE_ATTACK_TIME
            }
            .clamp(0.0, 1.0);

            if intensity <= 0.0 {
                continue;
            }

            let turbulence_noise = 0.66
                * (self.frication_noise_source)()
                * intensity as f64
                * self.glottis.get_noise_modulator() as f64;

            turbulence_noises.push((turbulence_noise, p.position as f64, p.diameter as f64));
        }

        for (noise, pos, diameter) in turbulence_noises.into_iter() {
            self.add_turbulence_noise_at_position(noise, pos, diameter);
        }
    }

    fn add_turbulence_noise_at_position(
        &mut self,
        turbulence_noise: f64,
        position: f64,
        diameter: f64,
    ) {
        let i = position.floor() as i64;
        let delta = position - i as f64;
        let thinnes0 = (8.0 * (0.7 - diameter)).clamp(0.0, 1.0);
        let openness = (30.0 * (diameter - 0.3)).clamp(0.0, 1.0);
        let noise0 = turbulence_noise * (1.0 - delta) * thinnes0 * openness;
        let noise1 = turbulence_noise * delta * thinnes0 * openness;
        if i + 1 < N as i64 {
            let idx = (i + 1) as usize;
            self.right[idx] += noise0 * 0.5;
            self.left[idx] += noise0 * 0.5;
        }
        if i + 2 < N as i64 {
            let idx = (i + 2) as usize;
            self.right[idx] += noise1 * 0.5;
            self.left[idx] += noise1 * 0.5;
        }
    }
}

fn assert_volume(val: f64) -> f64 {
    //assert!(val.abs() <= 1.0);
    val
}
