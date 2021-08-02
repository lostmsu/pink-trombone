use std::time::Duration;

use crate::{
    glottis::Glottis, noise::NoiseSource, tract::Tract, tract_shaper::TractShaper,
    turbulence::TurbulencePoint,
};

pub struct PinkTrombone {
    shaper: TractShaper,
    sample_rate: u32,
}

impl PinkTrombone {
    pub fn new(sample_rate: u32, rng: &mut dyn NoiseSource<f64>, seed: u16) -> PinkTrombone {
        if sample_rate >= u32::MAX / 2 {
            panic!("sample_rate too large");
        };
        if sample_rate == 0 {
            panic!("sample_rate must not be 0");
        }
        let glottis = Glottis::new(sample_rate, rng, seed);
        // tract runs at twice the sample rate
        let tract = Tract::new(glottis, 2 * sample_rate, rng);
        PinkTrombone {
            sample_rate,
            shaper: TractShaper::new(tract),
        }
    }

    const MAX_BLOCK_LEN: usize = 512;

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// How much time has already been generated.
    pub fn time(&self) -> Duration {
        Duration::from_secs_f32(self.tract().time)
    }

    // /// -1..+1
    //pub fn noise(self) -> f32 {}

    /// 0..1
    pub fn intensity(&self) -> f32 {
        self.glottis().intensity
    }
    pub fn set_intensity(&mut self, intensity: f32) {
        self.glottis_mut().intensity = intensity
    }

    /// 0..1
    pub fn loudness(&self) -> f32 {
        self.glottis().loudness
    }
    pub fn set_loudness(&mut self, loudness: f32) {
        self.glottis_mut().loudness = loudness
    }

    /// 0..
    pub fn target_frequency(&self) -> f32 {
        self.glottis().target_frequency
    }
    pub fn set_target_frequency(&mut self, target_frequency: f32) {
        self.glottis_mut().target_frequency = target_frequency
    }

    /// 0..1
    pub fn target_tenseness(&self) -> f32 {
        self.glottis().target_tenseness
    }
    pub fn set_target_tenseness(&mut self, target_tenseness: f32) {
        self.glottis_mut().target_tenseness = target_tenseness
    }

    /// 0..44 (see also Tract::n)
    pub fn tongue_index(&self) -> f64 {
        self.shaper.tongue_index
    }
    pub fn set_tongue_index(&mut self, tongue_index: f64) {
        self.shaper.tongue_index = tongue_index
    }

    /// 0..3(?)
    pub fn tongue_diameter(&self) -> f64 {
        self.shaper.tongue_diameter
    }
    pub fn set_tongue_diameter(&mut self, tongue_diameter: f64) {
        self.shaper.tongue_diameter = tongue_diameter
    }

    /// 0..
    pub fn vibrato_gain(&self) -> f32 {
        self.glottis().vibrato_amount
    }
    pub fn set_vibrato_gain(&mut self, vibrato_gain: f32) {
        self.glottis_mut().vibrato_amount = vibrato_gain
    }

    /// 0..
    pub fn vibrato_frequency(&self) -> f32 {
        self.glottis().vibrato_frequency
    }
    pub fn set_vibrato_frequency(&mut self, vibrato_frequency: f32) {
        self.glottis_mut().vibrato_frequency = vibrato_frequency
    }

    pub fn vibrato_wobble(&self) -> bool {
        self.glottis().auto_wobble
    }
    pub fn set_vibrato_wobble(&mut self, vibrato_wobble: bool) {
        self.glottis_mut().auto_wobble = vibrato_wobble
    }

    pub fn set_velum_open(&mut self, velum_open: bool) {
        self.shaper.set_velum_open(velum_open);
    }

    pub fn turbulence_points(&mut self) -> &mut Vec<TurbulencePoint> {
        &mut self.tract_mut().turbulence_points
    }

    /// Set `target_frequency` to the specified musical note.
    /// # Arguments
    /// * `semitone` - semitone, based at A4.
    pub fn set_musical_note(&mut self, semitone: f32) {
        self.glottis_mut().set_musical_note(semitone)
    }

    pub fn synthesize(&mut self, buf: &mut [f32]) {
        let mut p = 0;
        while p < buf.len() {
            let block_len = (buf.len() - p).min(PinkTrombone::MAX_BLOCK_LEN);
            let block_buf = &mut buf[p..p + block_len];
            self.synthesize_block(block_buf);
            p += block_len;
        }
    }

    pub fn reset(&mut self) {
        self.calculate_new_block_parameters(0.0);
    }

    fn synthesize_block(&mut self, buf: &mut [f32]) {
        let delta_time = buf.len() as f32 / self.sample_rate as f32;
        self.calculate_new_block_parameters(delta_time);
        for i in 0..buf.len() {
            let lambda1 = i as f64 / buf.len() as f64;
            let lambda2 = (i as f64 + 0.5) / buf.len() as f64;
            let glottal_output = self.glottis_mut().step(lambda1 as f32) as f64;
            let vocal1 = self.tract_mut().step(glottal_output, lambda1);
            let vocal2 = self.tract_mut().step(glottal_output, lambda2);
            buf[i] = (vocal1 + vocal2) * 0.125;
        }
    }

    fn calculate_new_block_parameters(&mut self, delta_time: f32) {
        self.glottis_mut().adjust_parameters(delta_time);
        self.shaper.adjust_tract_shape(delta_time as f64);
        self.tract_mut().calculate_new_block_parameters();
    }

    fn tract(&self) -> &Tract {
        &self.shaper.tract
    }

    fn glottis(&self) -> &Glottis {
        &self.tract().glottis
    }

    fn glottis_mut(&mut self) -> &mut Glottis {
        &mut self.tract_mut().glottis
    }

    fn tract_mut(&mut self) -> &mut Tract {
        &mut self.shaper.tract
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::rng::xorshift;

    const SAMPLE_RATE: u32 = 48000;
    const SEED: u16 = 9452;

    #[test]
    fn reproducible() {
        let mut random = xorshift::XorShift128::new(SEED.into());
        let mut trombone = PinkTrombone::new(SAMPLE_RATE, &mut random, SEED);
        let mut buffer = vec![0.0; SAMPLE_RATE as usize * 15];
        trombone.synthesize(&mut buffer);
        assert_eq!(format!("{:.10}", buffer.last().unwrap()), "0.0385491103");
    }
}
