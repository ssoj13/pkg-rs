use thiserror::Error;

#[derive(Debug, Error)]
pub enum MachoError {
    #[error("Mach-O parse error")]
    Parsing(#[from] goblin::error::Error),

    #[error("fat file is unix archive")]
    UnixArchive,

    #[error("unknown endianness")]
    UnknownEndian,

    #[error("remap string too long: need {0}, slot {1}")]
    RemapStringTooLong(usize, usize),

    #[error("buffer too short for in-place remap")]
    RemapBufferTooShort,
}
