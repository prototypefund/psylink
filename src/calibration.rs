// This should be the *only* file that interfaces with the burn library.

use burn::data::dataloader::batcher::Batcher;
use burn::data::dataloader::Dataset;
use burn::prelude::*;

const SAMPLE_TIMESPAN: usize = 250; // How many time frames should a training sample contain?

// The front end API
#[derive(Clone, Default, Debug)]
pub struct CalibController {
    pub dataset: PsyLinkDataset,
}

impl CalibController {
    pub fn add_packet(&mut self, sample: Vec<u8>) {
        self.dataset.all_packets.push(sample);
    }

    pub fn add_datapoint(&mut self, datapoint: Datapoint) {
        self.dataset.datapoints.push(datapoint);
    }

    // When you use this method, make sure to add the packet first.
    pub fn get_current_index(&self) -> usize {
        return self.dataset.all_packets.len();
    }
}

// This is a slim variant of a TrainingSample. It's faster to work with, but can't be
// used to train a NN directly. It's only valid in the context of a PsyLinkDataset,
// and PsyLinkDataset.get() will turn it into a TrainingSample when needed.
#[derive(Clone, Default, Debug)]
pub struct Datapoint {
    pub packet_index: usize,
    pub label: u8,
}

// This is a pair of features+labels that will be used for training the NN.
// It has a one-to-one mapping to a Datapoint struct.
#[derive(Clone, Default, Debug)]
pub struct TrainingSample {
    pub features: Vec<Vec<u8>>,
    pub label: u8,
}

// The dataset contains a list of all received packets in this session,
// along with datapoints which were recorded when the user was asked to
// perform a particular movement.
#[derive(Clone, Default, Debug)]
pub struct PsyLinkDataset {
    pub datapoints: Vec<Datapoint>,
    pub all_packets: Vec<Vec<u8>>,
}

impl Dataset<TrainingSample> for PsyLinkDataset {
    // Constructs a TrainingSample with training features that include
    // the signals at the time of recording, along with some amount of
    // signals from the past.
    fn get(&self, index: usize) -> Option<TrainingSample> {
        let datapoint = self.datapoints.get(index)?;

        if datapoint.packet_index < SAMPLE_TIMESPAN {
            return None;
        }
        let start = datapoint.packet_index - (SAMPLE_TIMESPAN - 1);
        let end = datapoint.packet_index;
        let packet = self.all_packets.get(start..=end)?;

        Some(TrainingSample {
            features: (*packet).iter().cloned().collect(),
            label: datapoint.label,
        })
    }
    fn len(&self) -> usize {
        return self.datapoints.len();
    }
}

#[derive(Clone, Debug)]
pub struct TrainingBatch<B: Backend> {
    // This is a 3D tensor with dimensions (sample number, time, channel)
    pub features: Tensor<B, 3>,

    // This is a 1D tensor with an array of labels, one for each of the samples
    pub targets: Tensor<B, 1, Int>,
}

#[derive(Clone)]
pub struct TrainingBatcher<B: Backend> {
    device: B::Device,
}

impl<B: Backend> TrainingBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}

impl<B: Backend> Batcher<TrainingSample, TrainingBatch<B>> for TrainingBatcher<B> {
    fn batch(&self, items: Vec<TrainingSample>) -> TrainingBatch<B> {
        let features = items
            .iter()
            .map(|item| Data::<u8, 2> {
                value: item.features.concat().iter().map(|&n| n).collect(),
                shape: Shape::<2> { dims: [250, 14] },
            })
            .map(|data| {
                Tensor::<B, 2>::from_data(data.convert(), &self.device).reshape([1, 250, 14])
            })
            .collect();

        let targets = items
            .iter()
            .map(|item| {
                Tensor::<B, 1, Int>::from_data(Data::from([item.label.elem()]), &self.device)
            })
            .collect();

        let features = Tensor::cat(features, 0).to_device(&self.device);
        let targets = Tensor::cat(targets, 0).to_device(&self.device);

        let batch = TrainingBatch { features, targets };
        return batch;
    }
}
