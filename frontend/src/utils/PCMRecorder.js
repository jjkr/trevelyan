/**
 * Simple PCM audio recorder that captures raw audio data directly
 * from the microphone without any encoding/decoding overhead
 */
class PCMRecorder {
  constructor() {
    this.audioContext = null;
    this.sourceNode = null;
    this.processorNode = null;
    this.stream = null;
    this.audioBuffer = [];
    this.isRecording = false;
    this.sampleRate = 16000; // Optimal for Whisper
  }

  /**
   * Start recording PCM audio
   */
  async start() {
    try {
      // Get microphone stream
      this.stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          sampleRate: this.sampleRate,
          channelCount: 1, // Mono
          echoCancellation: true,
          noiseSuppression: true,
        }
      });

      // Create audio context
      this.audioContext = new AudioContext({ sampleRate: this.sampleRate });
      
      // Create source from microphone stream
      this.sourceNode = this.audioContext.createMediaStreamSource(this.stream);
      
      // Create script processor for PCM capture
      // Using 4096 buffer size for good balance of latency and performance
      this.processorNode = this.audioContext.createScriptProcessor(4096, 1, 1);
      
      // Reset buffer
      this.audioBuffer = [];
      this.isRecording = true;

      // Capture PCM data
      this.processorNode.onaudioprocess = (event) => {
        if (!this.isRecording) return;
        
        const inputBuffer = event.inputBuffer;
        const inputData = inputBuffer.getChannelData(0); // Get mono channel
        
        // Copy the PCM data to our buffer
        const pcmChunk = new Float32Array(inputData.length);
        pcmChunk.set(inputData);
        this.audioBuffer.push(pcmChunk);
      };

      // Connect the audio graph
      this.sourceNode.connect(this.processorNode);
      this.processorNode.connect(this.audioContext.destination);

    } catch (error) {
      this.cleanup();
      throw error;
    }
  }

  /**
   * Stop recording and return the PCM data
   */
  stop() {
    this.isRecording = false;
    
    // Combine all audio chunks into a single Float32Array
    const totalLength = this.audioBuffer.reduce((sum, chunk) => sum + chunk.length, 0);
    const combinedBuffer = new Float32Array(totalLength);
    
    let offset = 0;
    for (const chunk of this.audioBuffer) {
      combinedBuffer.set(chunk, offset);
      offset += chunk.length;
    }

    this.cleanup();
    return combinedBuffer;
  }

  /**
   * Cancel recording and cleanup
   */
  cancel() {
    this.isRecording = false;
    this.cleanup();
  }

  /**
   * Cleanup audio resources
   */
  cleanup() {
    if (this.processorNode) {
      this.processorNode.disconnect();
      this.processorNode = null;
    }
    
    if (this.sourceNode) {
      this.sourceNode.disconnect();
      this.sourceNode = null;
    }
    
    if (this.audioContext && this.audioContext.state !== 'closed') {
      this.audioContext.close();
      this.audioContext = null;
    }
    
    if (this.stream) {
      this.stream.getTracks().forEach(track => track.stop());
      this.stream = null;
    }
    
    this.audioBuffer = [];
  }
}

export default PCMRecorder;
