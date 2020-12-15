// RIFF chunk (headers):
    // 4 byte ASCII identifier
    // 4 byte little endian 32 bit integer length
    // variable-sized field
    // a pad byte if the chunk's length is not even
    
// -"RIFF" and "LIST" chunks can contain subchunks (contain 4 byte ASCII identifier and rest subchunks?)
// -The file itself consists of one RIFF chunk (first four bytes should be "RIFF")

// LPCM (Linear pulse-code modulation)
    // "Linear" indicates linearly uniform quantization
    // Two properites: sampling rate, bit depth (bits per sample)

// TODO
    // panicky
    // Safety:
        // verify RIFF chunk
        // subchunks length always adds up to length of superchunk?
        // superchunks always 12 long before next chunk?

use std::fs;
use std::fmt;
use std::io::Write;
use std::fs::File;
use std::collections::HashMap;

fn main() {
    let data = fs::read("tones.wav").unwrap();
    
    let mut chunk_aggregator = Vec::new();
    let mut address = 0;
    loop {
        let next_chunk = &data[address..].to_vec();
        let chunk = Chunk::new(next_chunk, address);
        
        if chunk.length == 0 {
            break;
        }
        
        match chunk.superchunk {
            Some(_) => {
                address += 12;
            },
            None => {
                address += chunk.length as usize + 8;
            }
        }
        
        println!("{}", chunk);
        chunk_aggregator.push(chunk);
        if address >= data.len() {
            break;
        }
    }
    
    let wavfile = WavFile::new(chunk_aggregator);
    println!("{}", wavfile);
    wavfile.read_data();
}

struct Chunk {
    address: usize,
    superchunk: Option<String>,
    identifier: String,
    length: u32,
    var_field: Vec<u8> // the last byte may be a pad byte (depending if var field is even or odd in length)
}

// TODO does not handle wav files with compressed audio formats
struct WavFile {
    chunks: HashMap<String, Chunk>,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16
}

impl Chunk {
    fn new(data: &Vec<u8>, address: usize) -> Chunk {
        let mut identifier = String::from_utf8( data[0..4].to_vec() ).unwrap();
        let superchunk;
        let length = bytes_to_32bit(&data, 4);
        let var_field;
        
        if identifier == "RIFF" || identifier == "LIST" {
            superchunk = Some(identifier);
            identifier = String::from_utf8( data[8..12].to_vec() ).unwrap();
            var_field = Vec::new();
        } else {
            superchunk = None;
            var_field = data[8 .. 8 + length as usize].to_vec();
        }
        
        return Chunk {
            address,
            superchunk,
            identifier,
            length,
            var_field
        };
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(sc) = &self.superchunk {
            write!(f, "{} ", sc)?;
        }
        write!(f, "{}, length: {}", self.identifier, self.length)?;
        if self.length != 0 && self.length <= 256 {
            write!(f, ", data:")?;
            for byte in self.var_field.iter(){
                write!(f, " {:X}", byte)?;
            }
        }
        
        return Ok(());
    }
}

impl WavFile {
    fn new(chunks: Vec<Chunk>) -> WavFile {
        let mut chunks_map = HashMap::new();
        
        for chunk in chunks {
            chunks_map.insert(chunk.identifier.clone(), chunk);
        }
        
        let format = chunks_map.get(&String::from("fmt ")).unwrap();
        assert!(format.length == 16);
        let format = format.var_field.clone();
        assert!(bytes_to_16bit(&format[0..2].to_vec(), 0) == 1);
        
        let num_channels = bytes_to_16bit(&format[2..4].to_vec(), 0);
        let sample_rate = bytes_to_32bit(&format[4..8].to_vec(), 0);
        let byte_rate = bytes_to_32bit(&format[8..12].to_vec(), 0);
        let block_align = bytes_to_16bit(&format[12..14].to_vec(), 0);
        let bits_per_sample = bytes_to_16bit(&format[14..16].to_vec(), 0);
        
        return WavFile {
            chunks: chunks_map,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample
        };
    }
    
    fn read_data(&self) {
        
        const SIGNIFICANT_AMPLITUDE_DIVIDER: i32 = 64; // divide by this to get the significant amplitude threshold
        // divide by this to get the maximum difference between similar periods. 
            // if the resultant value is less than the significant amplitude, the significant amplitude is used instead
        const SIMILARITY_DIVIDER: i16 = 5; 
        const MAX_NUM_SAMPLES_DIFFERENCE: usize = 16; // the maximum difference in samples between approximately same contours
        const MAX_EQUILIBRIUM_PASSES: u8 = 4;
        
        let data_chunk = &self.chunks.get("data").unwrap();
        let data = &data_chunk.var_field;
        let sample_size = self.bits_per_sample as u32 / 8;
        let time_interval = 1.0 / self.sample_rate as f32;
        let num_samples = data.len() as u32 / sample_size;
        let alignment_offset = data_chunk.address % self.block_align as usize;
        
        // >:< assumes 16 bit sample size
        let significant_amplitude = ((1i32 << self.bits_per_sample) / SIGNIFICANT_AMPLITUDE_DIVIDER) as i16; // max sample size / 64
        
        let mut i = alignment_offset;
        let mut c1_contours = Vec::new();
        let mut c1_contours_start_time = Vec::new();
        // !!! let mut c2_contours = Vec::new();
        // let mut c2_contours_start_time = Vec::new();
                
        // analysis variables
        let mut analysis_failed_contours = Vec::new();
        // !!! analyze
            // waveforms which don't repeat
            // extracted waveforms match what I want
        
        if sample_size != 2 {
            println!("Sample size is unhandled.");
        }
        
        // get waveforms, which start with samples going from negative to positive and approximately repeat 
        if sample_size == 1 {
        } else if sample_size == 2 {
            
            let mut c1_waveform = Vec::new();
            let mut c1_period_idx = 0; // index of where the period of the waveform (may) be repeating from
            // wheteher or not a high enough amplitude was reached for the waveform to be considered significant
            let mut c1_significant = false; 
            let mut c1_last_sample = 0;
            let mut c1_num_equilibrium_passes = 0;
            
            // !!! let mut c2_waveform = Vec::new();
            let mut c2_period_idx = 0;
            let mut c2_significant = false;
            let mut c2_last_sample = 0;
            let mut c2_num_equilibrium_passes = 0;
            
            let check_period = |waveform: &Vec<i16>, period_idx: usize| -> bool {
                let mut first_idx = 0;
                let mut second_idx = period_idx;
                while first_idx < period_idx && second_idx < waveform.len() {
                    let first_abs = if waveform[first_idx] > 0 { waveform[first_idx] } else { -waveform[first_idx] };
                    let second_abs = if waveform[second_idx] > 0 { waveform[second_idx] } else { -waveform[second_idx] };
                    let larger = if first_abs > second_abs { first_abs } else { second_abs };
                    
                    let threshold = if (larger / SIMILARITY_DIVIDER) > significant_amplitude {
                        larger / SIMILARITY_DIVIDER 
                    } else { 
                        significant_amplitude
                    }; 
                    
                    if waveform[first_idx] - waveform[second_idx] > threshold 
                    || waveform[first_idx] - waveform[second_idx] < -threshold {
                        return false;
                    }
                    
                    first_idx += 1;
                    second_idx += 1;
                }
                
                return true;
            };
            
            while i < data.len() {
                let c1_this_sample = (bytes_to_16bit(&data, i)) as i16;
                let c2_this_sample = (bytes_to_16bit(&data, i+2)) as i16;
                
                if c1_waveform.len() != 0 {
                    c1_waveform.push(c1_this_sample);
                    
                    if c1_this_sample > significant_amplitude {
                        c1_significant = true;
                    }
                    
                    if c1_last_sample >= 0 && c1_this_sample < 0 && !c1_significant {
                        c1_waveform = Vec::new();
                        c1_period_idx = 0;
                        c1_num_equilibrium_passes = 1;
                    } else if c1_last_sample < 0 && c1_this_sample >= 0 {
                        if c1_period_idx == 0 {
                            c1_waveform.push(c1_this_sample);
                            c1_period_idx = c1_waveform.len() - 1;
                            c1_num_equilibrium_passes += 1;
                        } else {
                            if check_period(&c1_waveform, c1_period_idx) {
                                if c1_period_idx < c1_waveform.len() - c1_period_idx + MAX_NUM_SAMPLES_DIFFERENCE {
                                    c1_contours.push( c1_waveform[0 .. c1_period_idx].to_vec() );
                                    c1_contours_start_time.push( (i/4) as f32 * time_interval );
                                    c1_waveform = c1_waveform[c1_period_idx..].to_vec();
                                    c1_period_idx = c1_waveform.len() - 1;
                                    c1_num_equilibrium_passes = 0;
                                } else {
                                    c1_num_equilibrium_passes += 1;
                                }
                            } else {
                                c1_period_idx = c1_waveform.len() - 1;
                                c1_num_equilibrium_passes += 1;
                                
                                if c1_num_equilibrium_passes > MAX_EQUILIBRIUM_PASSES {
                                    // !!! even if part of the contour was a repeat of last period, the contour is discarded
                                        // does not check if sub contours repeat later on
                                        // may be of use to extract non-repeating contours as well
                                    analysis_failed_contours.push(c1_waveform);
                                    
                                    c1_waveform = Vec::new();
                                    c1_waveform.push(c1_this_sample);
                                    c1_period_idx = 0;
                                    c1_num_equilibrium_passes = 1;
                                    c1_significant = false;
                                }
                            }
                        }
                        
                    }
                } else if c1_last_sample < 0 && c1_this_sample >= 0 {
                    c1_waveform.push(c1_this_sample);
                    c1_period_idx = 0;
                    c1_significant = false;
                    c1_num_equilibrium_passes = 1;
                }
                
                c2_last_sample = c2_this_sample;
                c1_last_sample = c1_this_sample;
                i += 4;
            }
            println!();
        }
        
        println!("Data Chunk Address: {}", data_chunk.address);
        println!("Block Align: {}", self.block_align);
        println!("offset: {}", alignment_offset);
        println!("Track length: {}", num_samples as f32 * time_interval / self.num_channels as f32);
        println!("Number of contours: {}", c1_contours.len());
        println!("Number of failed contours: {}", analysis_failed_contours.len());
        
        // >:<
        // separate data extraction from exporting
        // magnitude of local maxima/minima. how to represent without each individual peak being represented? 
            // High dimensional... multiple derivatives may capture characteristics of multiple fluctuations?
            
        let mut output = String::from("time, note, num_smp, duration, num_ep, avg_mag_roc, avg_mag_acc, max_amp, min_amp, \
            num_max, num_min, d_above, d_below, d_inc, d_dec");
        
        for i in 0 .. c1_contours.len() {
            let contour = &c1_contours[i];
            let time = c1_contours_start_time[i];
            let note = (c1_contours_start_time[i] as u32 / 2) % 24 + 28; // 28 == c3
            let num_samples = contour.len();
            
            let mut duration = 0.0;
            let mut num_equilibrium_passes = 0;
            let mut total_magnitude_rate_of_change: u64 = 0; 
            let mut total_magnitude_acceleration: u64 = 0; 
            let mut max_amplitude = 0;
            let mut min_amplitude = 0; // >:< make floating point for compatibility regardless of sample bit depth
            let mut num_max = 0;
            let mut num_min = 0;
            let mut duration_above = 0.0;
            let mut duration_below = 0.0;
            let mut duration_increasing = 0.0;
            let mut duration_decreasing = 0.0;
            
            let mut last_change = 0;
            let mut rising = true;
            
            for j in 1 .. contour.len() {
                duration += time_interval;
                num_equilibrium_passes += if contour[j] >= 0 && contour[j-1] < 0 { 1 } else { 0 };
                
                let this_change = contour[j] - contour[j-1];
                let this_dchange = this_change - last_change;
                total_magnitude_rate_of_change += if this_change >= 0 { this_change as u64 } else { -this_change as u64 };
                total_magnitude_acceleration += if this_dchange >= 0 { this_dchange as u64 } else { -this_dchange as u64 };
                last_change = this_change;
                
                max_amplitude = if contour[j] > max_amplitude { contour[j] } else { max_amplitude };
                min_amplitude = if contour[j] < min_amplitude { contour[j] } else { min_amplitude };
                
                if rising && contour[j] < contour[j-1] {
                    rising = false;
                    num_max += 1;
                } else if !rising && contour[j] > contour[j-1] {
                    rising = true;
                    num_min += 1;
                }
                
                // TODO inexact if sample hits 0 but was above/below last sample
                duration_above += if contour[j] > 0 { time_interval } else { 0.0 }; 
                duration_below += if contour[j] < 0 { time_interval } else { 0.0 };
                duration_increasing += if rising { time_interval } else { 0.0 };
                duration_decreasing += if !rising { time_interval } else { 0.0 };
            }
            
            // let mut output = String::from("time, note, num_smp, duration, num_ep, avg_mag_roc, avg_mag_acc, max_amp, min_amp,\
            // num_max, num_min, d_above, d_below, d_inc, d_dec");
            output.push_str( &format!("\n{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                time, note, num_samples, duration, num_equilibrium_passes, total_magnitude_rate_of_change / num_samples as u64, 
                total_magnitude_acceleration / num_samples as u64, max_amplitude, min_amplitude, num_max, num_min, duration_above,
                duration_below, duration_increasing, duration_decreasing));
        }
        
        let mut file = File::create("contours.csv").unwrap();
        file.write(&output.into_bytes());
    }
}

impl fmt::Display for WavFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Number of channels: {}\n", self.num_channels)?;
        write!(f, "Sample rate: {}\n", self.sample_rate)?;
        write!(f, "Byte rate: {}\n", self.byte_rate)?;
        write!(f, "Block align: {}\n", self.block_align)?;
        write!(f, "Bits per sample: {}\n", self.bits_per_sample)?;
        
        return Ok(());
    }
}

// little-endian
fn bytes_to_16bit(bytes: &Vec<u8>, idx: usize) -> u16 {
    return 
        bytes[idx] as u16
        + bytes[idx + 1] as u16 * 0x100;
}

fn bytes_to_32bit(bytes: &Vec<u8>, idx: usize) -> u32 {
    return 
        bytes[idx] as u32 
        + bytes[idx + 1] as u32 * 0x100
        + bytes[idx + 2] as u32 * 0x100 * 0x100
        + bytes[idx + 3] as u32 * 0x100 * 0x100 * 0x100;
}


