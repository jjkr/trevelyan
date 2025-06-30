use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, SampleRate};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct AudioCapture {
    audio_receiver: Option<Receiver<Vec<f32>>>,
    is_recording: Arc<Mutex<bool>>,
    stream_handle: Option<JoinHandle<()>>,
}

impl AudioCapture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            audio_receiver: None,
            is_recording: Arc::new(Mutex::new(false)),
            stream_handle: None,
        })
    }

    pub fn start_recording(&mut self) -> Result<Receiver<Vec<f32>>, Box<dyn std::error::Error>> {
        if *self.is_recording.lock().unwrap() {
            return Err("Already recording".into());
        }

        let (sender, receiver) = unbounded::<Vec<f32>>();
        let is_recording = Arc::clone(&self.is_recording);
        
        *self.is_recording.lock().unwrap() = true;
        
        // Spawn a thread to handle audio capture
        let handle = thread::spawn(move || {
            if let Err(e) = Self::audio_capture_thread(sender, is_recording) {
                eprintln!("Audio capture error: {}", e);
            }
        });
        
        self.stream_handle = Some(handle);
        self.audio_receiver = Some(receiver.clone());
        
        Ok(receiver)
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.is_recording.lock().unwrap() = false;
        
        if let Some(handle) = self.stream_handle.take() {
            let _ = handle.join();
        }
        
        self.audio_receiver = None;
        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    fn audio_capture_thread(
        sender: Sender<Vec<f32>>,
        is_recording: Arc<Mutex<bool>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        
        // List all available input devices for debugging
        println!("Available input devices:");
        if let Ok(devices) = host.input_devices() {
            for (i, device) in devices.enumerate() {
                if let Ok(name) = device.name() {
                    println!("  {}: {}", i, name);
                    if let Ok(default_config) = device.default_input_config() {
                        println!("    Default config: {} channels, {} Hz", 
                                default_config.channels(), default_config.sample_rate().0);
                    }
                }
            }
        }
        
        let device = host.default_input_device()
            .ok_or("No input device available")?;

        println!("Selected audio capture device: {:?}", device.name());

        let config = Self::get_config(&device)?;
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;
        
        println!("Audio capture config: channels={}, sample_rate={}", channels, sample_rate);
        
        // Buffer for 1 second of audio at 16kHz
        let chunk_size = sample_rate as usize;
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(chunk_size)));
        let buffer_clone = Arc::clone(&buffer);
        let is_recording_clone = Arc::clone(&is_recording);
        let mut chunk_counter = 0;
        
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !*is_recording_clone.lock().unwrap() {
                    return;
                }

                let mut buf = buffer_clone.lock().unwrap();
                
                // Analyze incoming audio data for debugging
                let max_sample = data.iter().map(|&x| x.abs()).fold(0.0, f32::max);
                let avg_sample = data.iter().map(|&x| x.abs()).sum::<f32>() / data.len() as f32;
                
                // Convert to mono if needed and collect samples
                for frame in data.chunks(channels) {
                    let sample = if channels == 1 {
                        frame[0]
                    } else {
                        // Average all channels for mono conversion
                        frame.iter().sum::<f32>() / channels as f32
                    };
                    
                    buf.push(sample);
                    
                    // Send chunk when buffer is full (1 second of audio)
                    if buf.len() >= chunk_size {
                        chunk_counter += 1;
                        
                        // Analyze the chunk before sending
                        let chunk_max = buf.iter().map(|&x| x.abs()).fold(0.0, f32::max);
                        let chunk_avg = buf.iter().map(|&x| x.abs()).sum::<f32>() / buf.len() as f32;
                        
                        println!("Sending audio chunk #{}: {} samples, max_amp: {:.4}, avg_amp: {:.4}, callback_max: {:.4}, callback_avg: {:.4}", 
                                 chunk_counter, buf.len(), chunk_max, chunk_avg, max_sample, avg_sample);
                        
                        if let Err(_) = sender.send(buf.clone()) {
                            // Receiver dropped, stop recording
                            return;
                        }
                        buf.clear();
                    }
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        
        // Keep the stream alive while recording
        while *is_recording.lock().unwrap() {
            thread::sleep(std::time::Duration::from_millis(100));
        }
        
        drop(stream);
        Ok(())
    }

    fn get_config(device: &Device) -> Result<StreamConfig, Box<dyn std::error::Error>> {
        let supported_configs = device.supported_input_configs()?;
        
        // Try to find a config with 16kHz sample rate (optimal for Whisper)
        for config in supported_configs {
            if config.min_sample_rate() <= SampleRate(16000) && 
               config.max_sample_rate() >= SampleRate(16000) &&
               config.channels() >= 1 {
                return Ok(StreamConfig {
                    channels: 1, // Mono for Whisper
                    sample_rate: SampleRate(16000),
                    buffer_size: cpal::BufferSize::Default,
                });
            }
        }
        
        // Fallback to default config
        let _default_config = device.default_input_config()?;
        Ok(StreamConfig {
            channels: 1,
            sample_rate: SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        })
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stop_recording();
    }
}
