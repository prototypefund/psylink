// This should be the *only* file that interfaces with the burn library.

use burn::data::dataloader::Dataset;

// The front end API
#[derive(Clone, Default, Debug)]
pub struct CalibController {
    pub dataset: Samples,
}

impl CalibController {
    pub fn add_sample(&mut self, sample: Sample) {
        self.dataset.samples.push(sample);
    }
}

#[derive(Clone, Default, Debug)]
pub struct Sample {
    pub features: Vec<f64>,
    pub label: u8,
}

#[derive(Clone, Default, Debug)]
pub struct Samples {
    pub samples: Vec<Sample>,
}

impl Dataset<Sample> for Samples {
    fn get(&self, index: usize) -> Option<Sample> {
        return self.samples.get(index).cloned();
    }
    fn len(&self) -> usize {
        return self.samples.len();
    }
}
