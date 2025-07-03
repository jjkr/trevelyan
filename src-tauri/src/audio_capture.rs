use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use anyhow::Result;

const SAMPLE_RATE: u32 = 16000;
const BUFFER_DURATION_SECS: f32 = 30.0;
const BUFFER_SIZE: usize = (SAMPLE_RATE as f32 * BUFFER_DURATION_SECS) as usize;

// Thread-safe wrapper for audio data
#[derive(Clone)]
pub struct AudioBuffer {
    data: Arc<Mutex<VecDeque<f32>>>,
    is_recording: Arc<Mutex<bool>>,
}

impl AudioBuffer {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_SIZE))),
            is_recording: Arc::new(Mutex::new(false)),
        }
    }

    pub fn push_samples(&self, samples: &[f32]) {
        let recording = *self.is_recording.lock().unwrap();
        if !recording {
            return;
        }

        let mut buffer = self.data.lock().unwrap();
        for &sample in samples {
            buffer.push_back(sample);
            if buffer.len() > BUFFER_SIZE {
                buffer.pop_front();
            }
        }
    }

    pub fn get_data(&self) -> Vec<f32> {
        let buffer = self.data.lock().unwrap();
        buffer.iter().cloned().collect()
    }

    pub fn start_recording(&self) {
        let mut is_recording = self.is_recording.lock().unwrap();
        *is_recording = true;
    }

    pub fn stop_recording(&self) {
        let mut is_recording = self.is_recording.lock().unwrap();
        *is_recording = false;
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    pub fn get_audio_level(&self) -> f32 {
        let buffer = self.data.lock().unwrap();
        if buffer.is_empty() {
            return 0.0;
        }

        // Calculate RMS of recent samples for audio level
        let recent_samples: Vec<f32> = buffer.iter().rev().take(1600).cloned().collect(); // Last 100ms
        if recent_samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = recent_samples.iter().map(|&x| x * x).sum();
        (sum_squares / recent_samples.len() as f32).sqrt()
    }
}

// Global audio buffer that can be accessed from anywhere
static GLOBAL_AUDIO_BUFFER: std::sync::OnceLock<AudioBuffer> = std::sync::OnceLock::new();

#[derive(Clone)]
pub struct AudioCapture;

impl AudioCapture {
    pub fn new() -> Result<Self> {
        println!("🎙️ [audio_capture] Initializing audio capture...");
        
        let host = cpal::default_host();
        println!("🎙️ [audio_capture] Got default host: {:?}", host.id());
        
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
        println!("🎙️ [audio_capture] Got default input device: {:?}", device.name());

        let config = Self::get_config(&device)?;
        println!("🎙️ [audio_capture] Audio config: {:?}", config);
        
        // Initialize global buffer
        let buffer = GLOBAL_AUDIO_BUFFER.get_or_init(|| {
            println!("🎙️ [audio_capture] Initializing global audio buffer...");
            AudioBuffer::new()
        });
        
        // Build and start stream
        Self::build_stream(&device, &config, buffer.clone())?;
        println!("✅ [audio_capture] Audio capture initialized successfully");
        
        Ok(Self)
    }

    fn get_config(device: &Device) -> Result<StreamConfig> {
        let supported_configs_range = device
            .supported_input_configs()
            .map_err(|e| anyhow::anyhow!("Error querying configs: {}", e))?;

        let _supported_config = supported_configs_range
            .filter(|config| config.channels() == 1) // Mono audio
            .find(|config| {
                config.min_sample_rate().0 <= SAMPLE_RATE
                    && config.max_sample_rate().0 >= SAMPLE_RATE
            })
            .ok_or_else(|| anyhow::anyhow!("No suitable audio config found"))?;

        Ok(StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        })
    }

    fn build_stream(
        device: &Device,
        config: &StreamConfig,
        buffer: AudioBuffer,
    ) -> Result<()> {
        let stream = device.build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                buffer.push_samples(data);
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        
        // Keep the stream alive by leaking it (not ideal but necessary for this pattern)
        std::mem::forget(stream);
        
        Ok(())
    }

    pub fn start_recording(&self) -> Result<()> {
        if let Some(buffer) = GLOBAL_AUDIO_BUFFER.get() {
            buffer.start_recording();
            println!("Started audio recording");
        }
        Ok(())
    }

    pub fn stop_recording(&self) -> Result<()> {
        if let Some(buffer) = GLOBAL_AUDIO_BUFFER.get() {
            buffer.stop_recording();
            println!("Stopped audio recording");
        }
        Ok(())
    }

    pub fn get_buffer_data(&self) -> Vec<f32> {
        if let Some(buffer) = GLOBAL_AUDIO_BUFFER.get() {
            buffer.get_data()
        } else {
            Vec::new()
        }
    }

    pub fn is_recording(&self) -> bool {
        if let Some(buffer) = GLOBAL_AUDIO_BUFFER.get() {
            buffer.is_recording()
        } else {
            false
        }
    }

    pub fn get_audio_level(&self) -> f32 {
        if let Some(buffer) = GLOBAL_AUDIO_BUFFER.get() {
            buffer.get_audio_level()
        } else {
            0.0
        }
    }
}
