use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use borsh::io;
use borsh::BorshSerialize;
use pinocchio::{error::ProgramError, ProgramResult};

const MAX_EVENT_SIZE: usize = 256;
const MAX_BASE64_SIZE: usize = MAX_EVENT_SIZE.div_ceil(3) * 4;

struct SliceWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn written(&self) -> usize {
        self.pos
    }
}

impl<'a> io::Write for SliceWriter<'a> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let end = self.pos + data.len();
        if end > self.buf.len() {
            return Err(io::ErrorKind::WriteZero.into());
        }
        self.buf[self.pos..end].copy_from_slice(data);
        self.pos = end;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn emit<E: BorshSerialize>(event: &E) -> ProgramResult {
    let mut event_buf = [0u8; MAX_EVENT_SIZE];
    let mut writer = SliceWriter::new(&mut event_buf);

    event
        .serialize(&mut writer)
        .map_err(|_| ProgramError::BorshIoError)?;

    let event_len = writer.written();
    let event_bytes = &event_buf[..event_len];

    let mut b64_buf = [0u8; MAX_BASE64_SIZE];
    let encoded_len = STANDARD
        .encode_slice(event_bytes, &mut b64_buf)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let encoded_str = core::str::from_utf8(&b64_buf[..encoded_len])
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    pinocchio_log::log!("instruction data: {}", encoded_str);

    Ok(())
}
