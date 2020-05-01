use rustot::{
    jobs::FileDescription,
    ota::{
        ota::ImageState,
        pal::{OtaPal, OtaPalError, PalImageState},
    },
};
use std::io::{Cursor, Write};
use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub struct FileHandler {
    filebuf: Option<Cursor<Vec<u8>>>
}

impl FileHandler {
    pub fn new() -> Self {
        FileHandler {
            filebuf: None
        }
    }
}

impl OtaPal for FileHandler {
    type Error = ();
    fn abort(&mut self, _file: &FileDescription) -> Result<(), OtaPalError> {
        Ok(())
    }
    fn create_file_for_rx(&mut self, file: &FileDescription) -> Result<(), OtaPalError> {
        self.filebuf = Some(Cursor::new(Vec::with_capacity(file.filesize)));
        Ok(())
    }
    fn get_platform_image_state(&mut self) -> Result<PalImageState, OtaPalError> {
        unimplemented!()
    }
    fn set_platform_image_state(&mut self, _image_state: ImageState) -> Result<(), OtaPalError> {
        unimplemented!()
    }
    fn reset_device(&mut self) -> Result<(), OtaPalError> {
        Ok(())
    }
    fn close_file(&mut self, _file: &FileDescription) -> Result<(), OtaPalError> {
        if let Some(ref mut buf) = &mut self.filebuf {
            let mut hasher = Sha1::new();
            hasher.input(buf.get_ref());
            log::info!("Sha1 is {:}!", hasher.result_str());
            Ok(())
        } else {
            Err(OtaPalError::BadFileHandle)
        }
    }
    fn write_block(
        &mut self,
        _file: &FileDescription,
        block_offset: usize,
        block_payload: &[u8],
    ) -> Result<usize, OtaPalError> {
        if let Some(ref mut buf) = &mut self.filebuf {
            buf.set_position(block_offset as u64);
            buf.write(block_payload).map_err(|_e| OtaPalError::FileWriteFailed)?;
            Ok(block_payload.len())
        } else {
            Err(OtaPalError::BadFileHandle)
        }
    }
}
