use std::{
    error::Error,
    fs::File,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    num::NonZeroU8,
    path::PathBuf,
};

#[derive()]
pub struct Conductor {
    pub louie_swing: u8,
    pub bpm: u8,
    pub track_count: NonZeroU8,
    pub tracks: Vec<Track>,
}

impl Conductor {
    pub fn from_file(file: &PathBuf) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let file = File::open(file)?;
        let mut reader = BufReader::new(file);
        let mut metadata_buffer: [u8; 3] = [0; 3];
        reader.read_exact(&mut metadata_buffer)?;
        let [louie_swing, bpm, track_count] = metadata_buffer;

        // seek to the first byte of the first track
        reader.seek(SeekFrom::Current(21))?;
        let mut tracks = Vec::with_capacity(track_count.into());
        let mut byte_buffer: [u8; 4 * 9] = [0; 4 * 9];
        for i in 0..track_count {
            // read data block
            reader.read_exact(&mut byte_buffer)?;
            // extract initial data

            tracks.push(Track::from_bytes(&byte_buffer, i)?);
            match reader.seek(SeekFrom::Current(6 * 4)) {
                Ok(_) => (),
                Err(_) => break,
            };
        }

        Ok(Self {
            louie_swing,
            bpm,
            track_count: NonZeroU8::new(track_count).unwrap(),
            tracks,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub init_delay: u8,
    pub b_offset_flag: bool,
    pub q_offset_flag: bool,
    description: [u8; 8],
    pub track_copy: u8,
    pub echo: u8,
    pub ordered: bool,
    pub bank: Bank,
    pub program: u8,
    pub gesture_set: u8,
    pub timing: u8,
    pub gesture_count: u8,
    pub silent_count: u8,
    pub transposition: i8,
    pub volume: u8,
    pub panning: i8,
}

impl Track {
    pub fn from_bytes(
        byte_buffer: &[u8; 4 * 9],
        track_nr: u8,
    ) -> Result<Track, Box<dyn Error + Send + Sync>> {
        let mut cursor = Cursor::new(byte_buffer);

        let mut buffer = [0; 3];
        cursor.read_exact(&mut buffer)?;
        let [_, init_delay, b_offset_flag] = buffer;

        let mut description: [u8; 8] = [0; 8];
        cursor.read_exact(&mut description)?;

        let mut buffer = [0; 2];
        cursor.read_exact(&mut buffer)?;
        let [track_copy, echo] = buffer;

        cursor.set_position(cursor.position() + 8);

        let mut buffer = [0; 14];
        cursor.read_exact(&mut buffer)?;
        let [ordered, bank_byte, program, _, gesture_set, _, timing, gesture_count, silent_count, _, transposition, volume, panning, q_offset_flag] =
            buffer;

        let bank = match bank_byte {
            0 => Ok(Bank::Pikmin1SFX),
            1 => Ok(Bank::WatanabeSFX),
            2 => Ok(Bank::TotakaSFX),
            3 => Ok(Bank::HikinoSFX),
            4 => Ok(Bank::WakaiInstruments),
            5 => Ok(Bank::TotakaInstruments),
            _ => Err(format!("Error parsing Bank for track {}", track_nr + 1)),
        }?;

        Ok(Self {
            init_delay,
            b_offset_flag: (b_offset_flag == 1),
            q_offset_flag: (q_offset_flag == 1),
            description,
            track_copy,
            echo,
            ordered: (ordered == 1),
            bank,
            program,
            gesture_set,
            timing,
            gesture_count,
            silent_count,
            transposition: transposition as i8,
            volume,
            panning: (panning as i8) - 64,
        })
    }

    pub fn description(&self) -> &str {
        let error_message = "!!Description string is corrupted!!";
        let terminated_string = self.description.split(|n| *n == 205).next();

        match terminated_string {
            Some(bytes) => std::str::from_utf8(bytes).unwrap_or(error_message),
            None => error_message,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bank {
    Pikmin1SFX,
    WatanabeSFX,
    TotakaSFX,
    HikinoSFX,
    WakaiInstruments,
    TotakaInstruments,
}
