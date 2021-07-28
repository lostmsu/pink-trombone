mod glottis;
mod math;
mod noise;
mod noise_gen;
mod trombone;
mod tract;
mod tract_shaper;
mod transient;
mod turbulence;

pub use trombone::PinkTrombone;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
