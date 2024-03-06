use crate::ppu::{Queue, QueueNode};

////////////////////////////// WAVEFORMS /////////////////////////////
const DUTY: [[i8; 8]; 4] = [ // -1 = low, 1 = high, a volume unit of 0 is used when channel is off
    [-1, -1, -1, -1, -1, -1, -1, 1], // 12.5%
    [-1, -1, -1, -1, -1, -1, 1, 1], // 25%
    [-1, -1, -1, -1, 1, 1, 1, 1], // 50%
    [1, 1, 1, 1, 1, 1, -1, -1], // 75%
];

/////////////////////////////// APU ///////////////////////////////

///////////////////////////////////////////////////////////////////
// TODO:
// 1. Write function to check if sweep frequency overflow (> 2047) disable channel if true
// 2. Write triggers?
// 3. Write read / write functions
// DONE 4. Pattern match frame sequencer step to clock function units
// 5. Write Mixer
// 6. Sort out DAC
// 7. Implement SDL2 output of audio buffer
// 8. Read into why boytacean ticks via cycle count... confused at this, lol. I assume he does it for 4 cycles.
// 9. Sort out sequences for square channels (maybe channel 3 also?)
// 10. Tick channels
// 11. Finish channels 3 \ 4...
///////////////////////////////////////////////////////////////////
pub enum Channel {
    Chnl1,
    Chnl2,
    Chnl3,
    Chnl4,
}

pub struct APU {
    // Channels
    channel_1: Channel1,
    channel_2: Channel2,
    channel_3: Channel3,
    channel_4: Channel4,

    // Control
    master: u8,
    global_panning: u8,

    enabled: bool,
    left_enabled: bool,
    right_enabled: bool,

    sampling_rate: u16,
    channels: u8,

    // Sequencer and audio buffer
    sequencer: FrameSequencer,
    audio_buffer: Queue<u8>,
    audio_buffer_max: u32,
}

impl APU {
    pub fn new(sampling_rate: u16, channels: u8, buffer_size: u32) -> Self {
        APU {
            channel_1: Channel1::new(),
            channel_2: Channel2::new(),
            channel_3: Channel3::new(),
            channel_4: Channel4::new(),

            master: 0,
            global_panning: 0,

            enabled: true,
            left_enabled: true,
            right_enabled: true,

            sampling_rate,
            channels,

            sequencer: FrameSequencer::new(),
            audio_buffer: Queue::new(),
            audio_buffer_max: 0,
        }
    }

    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF10 => {
                ((self.channel_1.sweep.period & 0x07) << 4)
                | { if self.channel_1.sweep.direction_up { 0x00 } else { 0x80 } }
                | self.channel_1.sweep.shift & 0x7
                | 0x80
            },
            0xFF11 => {
                ((self.channel_1.duty & 0x03) << 6)
                | 0x3f
            },
            0xFF12 => {
                (self.channel_1.volume_envelope.initial_volume << 4)
                | { if self.channel_1.volume_envelope.direction_up { 0x80 } else { 0x00 } }
                | self.channel_1.volume_envelope.period & 0x7
            },
            0xFF13 => 0xFF,
            0xFF14 => {
                0xBF 
                | { if self.channel_1.length_ctr.enabled { 0x40 } else { 0x00 } }
            },
            0xFF15 => 0xFF,
            0xFF16 => {
                ((self.channel_2.duty & 0x03) << 6)
                | 0x3f    
            },
            0xFF17 => {
                (self.channel_2.volume_envelope.initial_volume << 4)
                | { if self.channel_2.volume_envelope.direction_up { 0x80 } else { 0x00 } }
                | self.channel_2.volume_envelope.period & 0x7
            },
            0xFF18 => 0xFF,
            0xFF19 => { 
                0xBF 
                | { if self.channel_2.length_ctr.enabled { 0x40 } else { 0x00 } }
            },
            0xFF1A => {
                0x7F
                | { if self.channel_3.dac_enabled { 0x80 } else { 0x00 } }
            },
            0xFF1B => {
                0xFF
            },
            0xFF1C => {
                (self.channel_3.volume & 0x03) << 5
                | 0x9F
            },
            0xFF1D => 0xFF,
            0xFF1E => { 
                0xBF 
                | { if self.channel_3.length_ctr.enabled { 0x40 } else { 0x00 } }
            },
            0xFF1F => 0xFF,
            0xFF20 => 0xFF,
            0xFF21 => {
                (self.channel_4.volume_envelope.initial_volume << 4)
                | { if self.channel_4.volume_envelope.direction_up { 0x80 } else { 0x00 } }
                | self.channel_4.volume_envelope.period & 0x7
            },
            _ => unreachable!()
        }
    }

    fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }

    // Tick the APU
    pub fn tick(&mut self) {
        if !self.enabled {
            return;
        }

        self.sequencer.tick();
        if self.sequencer.step != self.sequencer.last_step {
            match self.sequencer.step {
                0 => {
                    self.tick_all_length();
                },
                2 => {
                    self.tick_all_length();
                    self.tick_ch1_sweep();
                },
                4 => {
                    self.tick_all_length();
                },
                6 => {
                    self.tick_all_length();
                    self.tick_ch1_sweep();
                },
                7 => {
                    self.tick_all_envelopes();
                },
                _ => {},
            }
        }

        // Tick channels
        // Generate output,
    }

    // Tick all channel's Length Ctr
    pub fn tick_all_length(&mut self) {
        self.channel_1.length_ctr.tick();
        self.channel_2.length_ctr.tick();
        self.channel_3.length_ctr.tick();
        self.channel_4.length_ctr.tick();
    }

    // Tick channel 1's Sweep
    pub fn tick_ch1_sweep(&mut self) {
        self.channel_1.sweep.tick();
    }

    // Tick all Volume Envelopes
    pub fn tick_all_envelopes(&mut self) {
        self.channel_1.volume_envelope.tick();
        self.channel_2.volume_envelope.tick();
        self.channel_4.volume_envelope.tick();
    }
}

// Frame sequencer is responsible for clocking the function units of each channel
// It is ticked every t-cycle, and may clock the functio nunits, depending
// on its cycle. Mapping below.
// Step    Length Ctr    Vol Env    Sweep
// ---------------------------------------
//  0        Clock          -         -
//  1          -            -         -
//  2        Clock          -       Clock
//  3          -            -         -
//  4        Clock          -         -
//  5          -            -         -
//  6        Clock          -       Clock
//  7          -          Clock       -
// ---------------------------------------
pub struct FrameSequencer {
    cycles: u16,
    last_step: u8,
    step: u8,
}

impl FrameSequencer {
    pub fn new() -> Self {
        FrameSequencer {
            cycles: 0,
            last_step: 0,
            step: 0,
        }
    }

    fn tick(&mut self) {
        self.cycles += 1;

        self.last_step = self.step;
        if self.cycles == 8192 {
            if self.step != 7 {
                self.step += 1;
            } else {
                self.step = 0;
            }
            self.cycles = 0;
        }
    }
}
// Channels have a timer that details how many cycles until they output.
// Length ctr says how many cycles they live for,
// DAC enabled says whether or not they are to be DAC, where enabled says whether or not they are running
// Square wave 1 (with sweep)
pub struct Channel1 {
    timer: u16,
    enabled: bool,
    dac_enabled: bool,
    length_ctr: LengthCtr,
    volume_envelope: VolumeEnvelope,
    sweep: Sweep,

    duty: u8,
    sequence: u8,
    output: u8,
}

impl Channel1 {
    pub fn new() -> Self {
        Channel1 {
            timer: 0,
            enabled: false,
            dac_enabled: false,
            length_ctr: LengthCtr::new(64),
            volume_envelope: VolumeEnvelope::new(),
            sweep: Sweep::new(),
    
            duty: 0,
            sequence: 0,
            output: 0,
        }
    }

    fn tick(&mut self) {
        self.timer = self.timer.saturating_sub(1);
        if self.timer > 0 {
            return;
        }

        if self.enabled {
            self.output = if DUTY[self.duty as usize][self.sequence as usize] == 1 {
                self.volume_envelope.current_volume
            } else { 0 };
        } else {
            self.output = 0;
        }

        self.timer += ((2048 - self.sequence as u16) * 4) as u16;
        self.sequence = if self.sequence < 7 { 
            self.sequence + 1
        } else { 
            0
        };
    }
}

// Square wave 2 (without sweep)
pub struct Channel2 {
    timer: u16,
    enabled: bool,
    dac_enabled: bool,
    length_ctr: LengthCtr,
    volume_envelope: VolumeEnvelope,

    duty: u8,
    sequence: u8,
    output: u8,
}

impl Channel2 {
    pub fn new() -> Self {
        Channel2 {
            timer: 0,
            enabled: false,
            dac_enabled: false,
            length_ctr: LengthCtr::new(64),
            volume_envelope: VolumeEnvelope::new(),
    
            duty: 0,
            sequence: 0,
            output: 0,
        }
    }

    fn tick(&mut self) {
        self.timer = self.timer.saturating_sub(1);
        if self.timer > 0 {
            return;
        }

        if self.enabled {
            self.output = if DUTY[self.duty as usize][self.sequence as usize] == 1 {
                self.volume_envelope.current_volume
            } else { 0 };
        } else {
            self.output = 0;
        }

        self.timer += ((2048 - self.sequence as u16) * 4) as u16;
        self.sequence = if self.sequence < 7 { 
            self.sequence + 1
        } else { 
            0
        };
    }
}

// Predetermined (in ROM) sequences
pub struct Channel3 {
    timer: u16,
    enabled: bool,
    dac_enabled: bool,
    length_ctr: LengthCtr,
    volume: u8,
    // The RAM to be used for generating waves for Channel 3
    wave_ram: [u8; 16],
    output: u8,
}

impl Channel3 {
    pub fn new() -> Self {
        Channel3 {
            timer: 0,
            enabled: false,
            dac_enabled: false,
            length_ctr: LengthCtr::new(256),
            volume: 0,

            wave_ram: [0_u8; 16],
            output: 0,
        }
    }

    fn tick(&mut self) {
        self.timer = self.timer.saturating_sub(1);
        if self.timer > 0 {
            return;
        }
    }
}

// Noise channel
pub struct Channel4 {
    timer: u16,
    enabled: bool,
    dac_enabled: bool,
    length_ctr: LengthCtr,
    volume_envelope: VolumeEnvelope,

    divisor_code: u8,
    shift: u8,
    counter_width: u8,

    output: u8,
}

impl Channel4 {
    pub fn new() -> Self {
        Channel4 {
            timer: 0,
            enabled: false,
            dac_enabled: false,
            length_ctr: LengthCtr::new(64),
            volume_envelope: VolumeEnvelope::new(),

            divisor_code: 0,
            shift: 0,
            counter_width: 0,

            output: 0,
        }
    }

    fn tick(&mut self) {
        self.timer.saturating_sub(1);
        if self.timer > 0 {
            return;
        }
        
        self.timer = match self.divisor_code {
            0 => { 8 << self.shift },
            1 => { 16 << self.shift },
            2 => { 32 << self.shift },
            3 => { 48 << self.shift },
            4 => { 64 << self.shift },
            5 => { 80 << self.shift },
            6 => { 96 << self.shift },
            7 => { 112 << self.shift },
            _ => unreachable!(),
        };
    }
}

// Each channel has an associated length that details its runtime
// Length Counter disables a channel once its length has been reached (length_timer == 0)
// Length = max_length - rLength, where max length is 64 for all channels, except channel 3 it is 256
// On length trigger, length_timer set to max length, ONLY if is 0
pub struct LengthCtr {
    enabled: bool,
    max_length: u16,
    length_timer: u16,
}

impl LengthCtr {
    pub fn new(max: u16) -> Self {
        LengthCtr {
            enabled: false,
            max_length: max,
            length_timer: 0,
        }
    }

    fn trigger(&mut self) {
        if self.length_timer == 0 {
            self.length_timer = 64;
        }
    }

    fn channel_active(&mut self) -> bool {
        self.length_timer > 0
    }

    fn tick(&mut self) {
        if self.length_timer == 0 || !self.enabled {
            return;
        } else {
            self.length_timer = self.length_timer.saturating_sub(1);
        }
    }
}

// Sweep function periodically adjusts frequency (pitch), only used by channel 1 (square wave)
// Controlled by NR10 register
pub struct Sweep {
    enabled: bool,
    frequency: u16,
    shadow_frequency: u16,
    sweep_timer: u8,
    period: u8,
    direction_up: bool,
    shift: u8,
}

impl Sweep {
    pub fn new() -> Self {
        Sweep {
            enabled: false,
            frequency: 0,
            shadow_frequency: 0,
            sweep_timer: 0,
            period: 0,
            direction_up: false,
            shift: 0,
        }
    }

    fn tick(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer = self.sweep_timer.saturating_sub(1);
        }

        if self.sweep_timer == 0 {
            if self.period > 0 {
                self.sweep_timer = self.period;
            } else {
                self.sweep_timer = 8;
            }

            if self.enabled && self.period > 0 {
                let mut new_frequency = self.calculate_frequency();

                if self.frequency <= 2047 && self.shift > 0 {
                    self.frequency = new_frequency;
                    self.shadow_frequency = new_frequency;

                    // Overflow check
                }
            }
        }
    }

    fn calculate_frequency(&mut self) -> u16 {
        let mut new_frequency = self.shadow_frequency >> self.shift;

        if !self.direction_up {
            new_frequency = self.shadow_frequency - self.frequency;
        } else {
            new_frequency = self.shadow_frequency - self.frequency;
        }

        return new_frequency;
    }
}

// Volume envelope function periodically adjusts volume, used by all channels except 3
// Controlled by NRx2 registers (NR12, 22, 42)
pub struct VolumeEnvelope {
    enabled: bool,
    initial_volume: u8,
    current_volume: u8,
    direction_up: bool,
    period_timer: u8,
    period: u8,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        VolumeEnvelope {
            enabled: false,
            initial_volume: 0,
            current_volume: 0,
            direction_up: false,
            period_timer: 0,
            period: 0,
        }
    }

    pub fn tick(&mut self) {
        if !self.enabled || self.period == 0 {
            return;
        }
        if self.period_timer > 0 {
            self.period_timer = self.period_timer.saturating_sub(1);
        }
        if self.period_timer == 0 {
            self.period_timer = self.period;

            if (self.current_volume < 0xF && self.direction_up) || (self.current_volume > 0x0 && !self.direction_up) {
                if self.direction_up {
                    self.current_volume = self.current_volume.saturating_add(1);
                } else {
                    self.current_volume = self.current_volume.saturating_sub(1);
                }
            }
        }
    }
}
