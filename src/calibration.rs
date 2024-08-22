// This should be the *only* file that interfaces with the burn library.

use burn::backend::{Autodiff, Wgpu};
use burn::data::dataloader::batcher::Batcher;
use burn::data::dataloader::{DataLoaderBuilder, Dataset};
use burn::nn::{
    conv::{Conv2d, Conv2dConfig},
    loss::CrossEntropyLoss,
    pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
    Dropout, DropoutConfig, Linear, LinearConfig, Relu,
};
use burn::optim::AdamConfig;
use burn::prelude::*;
use burn::record::BinFileRecorder;
use burn::record::BinBytesRecorder;
use burn::record::Recorder;
use burn::record::CompactRecorder;
use burn::record::FullPrecisionSettings;
use burn::tensor::backend::AutodiffBackend;
use burn::train::{
    metric::{AccuracyMetric, LossMetric},
    ClassificationOutput, LearnerBuilder, TrainOutput, TrainStep, ValidStep,
};
use rand::seq::SliceRandom;
use rand::thread_rng;

const VALIDATION_SET_PERCENTAGE: usize = 20;
const SAMPLE_TIMESPAN: usize = 250; // How many time frames should a training sample contain?
pub const TEST_DATASET: ([(usize, u8); 12450], [[u8; 14]; 20475]) = include!("data/test_dataset.rs");
pub const TEST_MODEL: &[u8] = include_bytes!("data/test_model.bin");

// The front end API
#[derive(Clone, Default, Debug)]
pub struct CalibController {
    pub dataset: PsyLinkDataset,
}

pub type DefaultBackend = Autodiff<Wgpu>;
pub type DefaultModel = Model<Autodiff<Wgpu>>;

impl CalibController {
    pub fn add_packet(&mut self, sample: Vec<u8>) {
        self.dataset.all_packets.push(sample);
    }

    pub fn add_datapoint(&mut self, datapoint: Datapoint) {
        self.dataset.datapoints.push(datapoint);
    }

    pub fn reset(&mut self) {
        self.dataset.datapoints.clear();
        self.dataset.all_packets.clear();
    }

    // When you use this method, make sure to add the packet first.
    pub fn get_current_index(&self) -> usize {
        return self.dataset.all_packets.len();
    }

    fn create_artifact_dir(artifact_dir: &str) {
        // Remove existing artifacts before to get an accurate learner summary
        std::fs::remove_dir_all(artifact_dir).ok();
        std::fs::create_dir_all(artifact_dir).ok();
    }

    pub fn infer_latest(&self, model: DefaultModel) -> Option<i32> {
        let item = self.dataset.get_latest()?;
        Some(infer_item(model, item))
    }

    pub fn train(&self) -> Result<DefaultModel, Box<dyn std::error::Error>> {
        //type MyBackend = Wgpu<f32, i32>;

        // Create a default Wgpu device
        let device = burn::backend::wgpu::WgpuDevice::default();

        // All the training artifacts will be saved in this directory
        let artifact_dir = "/tmp/psylink";

        // Train the model
        self.train2::<DefaultBackend>(
            artifact_dir,
            TrainingConfig::new(ModelConfig::new(), AdamConfig::new()),
            device.clone(),
        )
    }

    fn train2<B: AutodiffBackend>(
        &self,
        artifact_dir: &str,
        config: TrainingConfig,
        device: B::Device,
    ) -> Result<Model<B>, Box<dyn std::error::Error>> {
        Self::create_artifact_dir(artifact_dir);
        config
            .save(format!("{artifact_dir}/config.json"))
            .expect("Config should be saved successfully");

        B::seed(config.seed);

        println!("Dataset length: {}", self.dataset.len());

        // Build dataset
        let (dataset_train, dataset_valid) = self.dataset.split_train_validate();

        // Build batchers
        let batcher_train = TrainingBatcher::<B>::new(device.clone());
        let batcher_valid = TrainingBatcher::<B::InnerBackend>::new(device.clone());

        // Build data loaders
        let dataloader_train = DataLoaderBuilder::new(batcher_train)
            .batch_size(config.batch_size)
            .shuffle(config.seed)
            .num_workers(config.num_workers)
            .build(dataset_train);

        let dataloader_test = DataLoaderBuilder::new(batcher_valid)
            .batch_size(config.batch_size)
            .shuffle(config.seed)
            .num_workers(config.num_workers)
            .build(dataset_valid);

        // Build learner
        let learner = LearnerBuilder::new(artifact_dir)
            .metric_train_numeric(AccuracyMetric::new())
            .metric_valid_numeric(AccuracyMetric::new())
            .metric_train_numeric(LossMetric::new())
            .metric_valid_numeric(LossMetric::new())
            .with_file_checkpointer(CompactRecorder::new())
            .devices(vec![device.clone()])
            .num_epochs(config.num_epochs)
            .summary()
            .build(
                config.model.init::<B>(&device),
                config.optimizer.init(),
                config.learning_rate,
            );

        // Fit the learner
        let model_trained = learner.fit(dataloader_train, dataloader_test);

        model_trained
            .clone()
            .save_file(format!("{artifact_dir}/model"), &CompactRecorder::new())
            .expect("Trained model should be saved successfully");

        let recorder = BinFileRecorder::<FullPrecisionSettings>::new();
        model_trained
            .clone()
            .save_file(format!("{artifact_dir}/model_bin"), &recorder)
            .expect("Should be able to save the model");
        Ok(model_trained)
    }
}

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
    pool: AdaptiveAvgPool2d,
    dropout: Dropout,
    linear1: Linear<B>,
    linear2: Linear<B>,
    activation: Relu,
}

impl<B: AutodiffBackend> TrainStep<TrainingBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, batch: TrainingBatch<B>) -> TrainOutput<ClassificationOutput<B>> {
        // TODO: is "item" the right name for this variable?...
        let item = self.forward_classification(batch.features, batch.targets);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<TrainingBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, batch: TrainingBatch<B>) -> ClassificationOutput<B> {
        self.forward_classification(batch.features, batch.targets)
    }
}

impl<B: Backend> Model<B> {
    /// # Shapes
    ///   - Features [batch_size, height, width]
    ///   - Output [batch_size, num_classes]
    pub fn forward(&self, features: Tensor<B, 3>) -> Tensor<B, 2> {
        let [batch_size, height, width] = features.dims();

        // Create a channel at the second dimension.
        let x = features.reshape([batch_size, 1, height, width]);

        let x = self.conv1.forward(x); // [batch_size, 8, _, _]
        let x = self.dropout.forward(x);
        let x = self.conv2.forward(x); // [batch_size, 16, _, _]
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);

        let x = self.pool.forward(x); // [batch_size, 16, 8, 8]
        let x = x.reshape([batch_size, 16 * 8 * 8]);
        let x = self.linear1.forward(x);
        let x = self.dropout.forward(x);
        let x = self.activation.forward(x);

        self.linear2.forward(x) // [batch_size, num_classes]
    }

    pub fn forward_classification(
        &self,
        features: Tensor<B, 3>,
        targets: Tensor<B, 1, Int>,
    ) -> ClassificationOutput<B> {
        let output = self.forward(features);
        let loss =
            CrossEntropyLoss::new(None, &output.device()).forward(output.clone(), targets.clone());

        ClassificationOutput::new(loss, output, targets)
    }
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    #[config(default = "2")]
    num_classes: usize,
    #[config(default = "32")]
    hidden_size: usize,
    #[config(default = "0.5")]
    dropout: f64,
}

impl ModelConfig {
    /// Returns the initialized model.
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            conv1: Conv2dConfig::new([1, 16], [5, 5]).init(device),
            conv2: Conv2dConfig::new([16, 16], [5, 5]).init(device),
            pool: AdaptiveAvgPool2dConfig::new([8, 8]).init(),
            activation: Relu::new(),
            linear1: LinearConfig::new(16 * 8 * 8, self.hidden_size).init(device),
            linear2: LinearConfig::new(self.hidden_size, self.num_classes).init(device),
            dropout: DropoutConfig::new(self.dropout).init(),
        }
    }
}

#[derive(Config)]
pub struct TrainingConfig {
    pub model: ModelConfig,
    pub optimizer: AdamConfig,
    #[config(default = 10)]
    pub num_epochs: usize,
    #[config(default = 32)]
    pub batch_size: usize,
    #[config(default = 8)]
    pub num_workers: usize,
    #[config(default = 42)]
    pub seed: u64,
    #[config(default = 1.0e-4)]
    pub learning_rate: f64,
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

impl PsyLinkDataset {
    fn split_train_validate(&self) -> (Self, Self) {
        let mut datapoints = self.datapoints.clone();
        let mut rng = thread_rng();
        datapoints.shuffle(&mut rng);

        let validation_split_index = (datapoints.len() * VALIDATION_SET_PERCENTAGE) / 100;
        let training_datapoints = if validation_split_index <= datapoints.len() {
            datapoints.split_off(validation_split_index)
        } else {
            vec![]
        };

        let train_dataset = PsyLinkDataset {
            datapoints: training_datapoints,
            all_packets: self.all_packets.clone(),
        };
        let validation_dataset = PsyLinkDataset {
            datapoints,
            all_packets: self.all_packets.clone(),
        };

        // TODO: actually do the splitting
        (train_dataset, validation_dataset)
    }

    pub fn to_string(&self) -> String {
        let mut string = String::new();
        string += "([\n";
        for datapoint in &self.datapoints {
            string += format!("({},{}),", datapoint.packet_index, datapoint.label).as_str();
        }
        string += "],\n[\n";
        for packet in &self.all_packets {
            string += "[";
            for byte in packet {
                string += byte.to_string().as_str();
                string += ",";
            }
            string += "],\n";
        }
        string += "])\n";
        string
    }

    pub fn from_arrays(datapoints: &[(usize, u8)], all_packets: &[[u8; 14]]) -> Self {
        let datapoints: Vec<Datapoint> = datapoints
            .iter()
            .map(|d| Datapoint {
                packet_index: d.0,
                label: d.1,
            })
            .collect();

        let all_packets: Vec<Vec<u8>> = all_packets
            .iter()
            .map(|packet| packet.iter().map(|&byte| byte).collect())
            .collect();

        Self {
            datapoints,
            all_packets,
        }
    }

    pub fn get_latest(&self) -> Option<TrainingSample> {
        let len: usize = self.len();
        if len == 0 {
            return None;
        }
        Some(self.get(len - 1)?)
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

pub fn load_test_model() -> Model<DefaultBackend> {
    let device = burn::backend::wgpu::WgpuDevice::default();
    let config = TrainingConfig::load_binary(include_bytes!("data/test_model_config.json"))
        .expect("Config should exist for the model");
    let record = BinBytesRecorder::<FullPrecisionSettings>::default()
        .load(TEST_MODEL.to_vec(), &device)
        .expect("Should be able to load model the model weights from bytes");
    let model = config.model.init::<DefaultBackend>(&device).load_record(record);
    model
}

pub fn infer_item(model: Model<DefaultBackend>, item: TrainingSample) -> i32 {
    let device = burn::backend::wgpu::WgpuDevice::default();
    let batcher = TrainingBatcher::<DefaultBackend>::new(device.clone());
    let batch = batcher.batch(vec![item]);
    let output = model.forward(batch.features);
    let predicted = output.argmax(1).flatten::<1>(0, 1).into_scalar();
    return predicted;
}

pub fn train() -> Result<(), Box<dyn std::error::Error>> {
    let mut calib = CalibController::default();
    calib.dataset = PsyLinkDataset::from_arrays(&TEST_DATASET.0, &TEST_DATASET.1);
    calib.train()?;

    Ok(())
}

pub fn infer() -> Result<(), Box<dyn std::error::Error>> {
    let model = load_test_model();
    let dataset = PsyLinkDataset::from_arrays(&TEST_DATASET.0, &TEST_DATASET.1);

    for item in dataset.iter() {
        let predicted = infer_item(model.clone(), item);
        dbg!(predicted);
    }

    Ok(())
}
