//! This is (intended to be) a small but helpful codec abstraction
//! for the program to use. The GUI itself and the user shouldn't have
//! to care which decoder or encoder is being used exactly.

use std::{
	cell::{RefCell, Ref},
	io::Cursor, num::NonZeroUsize
};
#[allow(unused_imports)]
use log::{ trace, debug, info, warn, error };
use rodio::Source;

/// Enum of encoded file types.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EncodingType {
	/// Ogg Vorbis audio.
	Ogg,
	/// FLAC audio.
	FLAC,
	/// WAV audio.
	WAV,
	/// MP3 audio.
	MP3,
	/// Special encoding type that specifies data which
	/// could or should not be read as audio.
	Bin
}

impl EncodingType {
	/// Whether or not this encoding type could be decoded
	/// with the current feature set.
	/// 
	/// This doesn't necessarily mean the decoding will be
	/// successful, just that an encoding of this type
	/// should be usabale.
	pub fn can_be_decoded(&self) -> bool {
		match self {
			Self::Bin => false,
			_ => true
		}
	}

	/// Return an EncodingType from the given extension.
	pub fn from_extension(extension: &str) -> Self {
		match &extension.to_lowercase()[..] {
			"ogg" => Self::Ogg,
			"flac" => Self::FLAC,
			"wav" => Self::WAV,
			"mp3" => Self::MP3,
			_ => Self::Bin
		}
	}
}

/// An encoded file.
pub struct EncodedFile {
	pub bytes: Vec<u8>,
	pub encoding: EncodingType,
	channels: RefCell<Option<u16>>,
	sample_rate: RefCell<Option<u32>>
}

impl EncodedFile {
	/// Create a new encoded file from the given bytes.
	pub fn from_bytes_with_encoding(bytes: Vec<u8>, encoding: EncodingType) -> Self {
		Self {
			bytes,
			encoding,
			channels: RefCell::new(None),
			sample_rate: RefCell::new(None)
		}
	}

	/// Whether or not this file could be decoded with
	/// the current feature set.
	/// 
	/// This doesn't necessarily mean the decoding will be
	/// successful, just that an encoding of this type
	/// should be usabale.
	pub fn can_be_decoded(&self) -> bool {
		self.encoding.can_be_decoded()
	}

	/// Attempt to use rodio to decode this.
	/// 
	/// This function is, hopefully, temporary.
	pub fn rodio_decode(&self) -> Result<Vec<i16>, rodio::decoder::DecoderError> {
		let cursor = Cursor::new(self.bytes.clone());

		// It would be nice to use Kira for this,
		// but it seems to coerce everything into dual channel,
		// which isn't ideal.
		// 
		// I've considered using the Symphonia crate, but it looks far
		// too complicated to use for something otherwise so simple.
		// For example, this is the basic "decoding audio" mock-up:
		// https://github.com/pdeljanov/Symphonia/blob/master/symphonia/examples/getting-started.rs#L53
		let decoder = rodio::Decoder::new(cursor);
		if let Err(error) = decoder {
			// This can't be decoded
			return Err(error)
		};
		let decoder = decoder.unwrap();

		let _ = self.channels.borrow_mut().replace(decoder.channels());
		let _ = self.sample_rate.borrow_mut().replace(decoder.sample_rate());

		Ok(decoder.collect())
	}

	/// Attempt to use rodio to decode this, and then resample if needed.
	/// 
	/// This function is, hopefully, temporary.
	pub fn rodio_decode_resample(&self) -> Result<Vec<i16>, rodio::decoder::DecoderError> {
		let mut decoded = self.rodio_decode()?;

		let decoder_sample_rate = self.sample_rate.borrow().expect("sample rate with decoded audio file");
		// The lopus format only supports these sample rates
		let sample_rate = if decoder_sample_rate <= 8_000 {8_000}
		else if decoder_sample_rate <= 12_000 {12_000}
		else if decoder_sample_rate <= 16_000 {16_000}
		else if decoder_sample_rate <= 24_000 {24_000}
		else {48_000};

		let channel_count = if self.channels.borrow().expect("channels with decoded audio file") == 1 { 1 } else { 2 };

		if decoder_sample_rate != sample_rate {
			// Need to resample
			if channel_count == 1 {
				let input = fon::Audio::<fon::chan::Ch16, 1>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 1>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			} else {
				let input = fon::Audio::<fon::chan::Ch16, 2>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 2>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			}
		}

		Ok(decoded)
	}

	/// Decode this file, and then convert to WAV.
	pub fn to_wav(&self, end: Option<NonZeroUsize>) -> Result<Vec<u8>, DecodeError> {
		let wav = match self.encoding {
			EncodingType::Bin => return Err(DecodeError::DecodeBin),
			_ => {
				let raw = match self.rodio_decode() {
					Ok(r) => r,
					Err(error) => return Err(DecodeError::RodioDecoder(error))
				};
				// Make the header
				let header = wav::Header::new(wav::WAV_FORMAT_PCM, self.channels.borrow().unwrap(), self.sample_rate.borrow().unwrap(), 16);
				// Create the empty vec
				let mut wav_file: Vec<u8> = Vec::new();
				// And the cursor to write to it
				let mut wav_cursor = Cursor::new(&mut wav_file);
				// Get the raw slice if there is a specific sample limit
				let raw = &raw[0..if let Some(end) = end {
					let sample_length = usize::from(end) * self.channels.borrow().unwrap() as usize;
					if raw.len() < sample_length { raw.len() } else { sample_length }
				} else {
					raw.len()
				}];
				// Finally, write the wav file
				// I don't honestly know when this can fail...
				if let Err(error) = wav::write(header, &wav::BitDepth::Sixteen(raw.to_vec()), &mut wav_cursor) { return Err(DecodeError::IO(error))};

				wav_file
			}
		};

		debug!("Got wav file from raw audio (wav size is {})", crate::util::human_readable_size(wav.len() as u64));

		Ok(wav)
	}

	pub fn encode(&self, encoding: EncodingType) -> Result<Vec<u8>, EncodeError> {
		match encoding {
			EncodingType::Bin => if encoding == self.encoding { Ok(self.bytes.clone()) } else { Err(EncodeError::EncodeBin) },
			EncodingType::WAV => match self.to_wav(None) {
				Ok(bytes) => Ok(bytes),
				Err(decode_error) => Err(EncodeError::DecodeError(decode_error))
			},
			_ => todo!()
		}
	}
}

/// Decoder errors.
pub enum DecodeError {
	/// Attempted to decode a file whose encoding
	/// was not known.
	DecodeBin,
	/// rodio returned an error.
	RodioDecoder (rodio::decoder::DecoderError),
	IO (std::io::Error)
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			Self::DecodeBin => write!(f, "Can't decode a bin, which is not a real encoding"),
			Self::RodioDecoder(rodio_error) => rodio_error.fmt(f),
			Self::IO(io_error) => io_error.fmt(f)
		}
    }
}

/// Encoder errors.
pub enum EncodeError {
	/// Attempted to encode to bin, which is not
	/// a real encoding.
	EncodeBin,
	/// The file couldn't be decoded.
	DecodeError (DecodeError)
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			Self::EncodeBin => write!(f, "Can't encode to bin, which is not a real encoding"),
			Self::DecodeError(decode_error) => write!(f, "Error decoding: {}", decode_error)
		}
    }
}
