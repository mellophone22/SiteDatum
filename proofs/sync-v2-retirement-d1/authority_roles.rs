//! Experimental D1 role contract, not a production API or a vault implementation.
//! No key removal, storage, networking, or serialization capability.
#![allow(dead_code)]

mod sealed {
    pub trait Role {}
}

pub trait ReadBackend {
    type Error;
    fn select(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
        version: u32,
    ) -> Result<&[u8; 32], Self::Error>;
}

pub trait WriteBackend {
    type Error;
    fn current(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
    ) -> Result<(u32, &[u8; 32]), Self::Error>;
}

pub trait ReadKeys: sealed::Role {
    type Error;
    fn read(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
        version: u32,
    ) -> Result<&[u8; 32], Self::Error>;
}

pub trait WriteKeys: sealed::Role {
    type Error;
    fn for_current_write(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
    ) -> Result<(u32, &[u8; 32]), Self::Error>;
}

pub struct HistoricalReader<B: ReadBackend> {
    backend: B,
}

pub struct ActiveWriter<B: WriteBackend> {
    backend: B,
}

impl<B: ReadBackend> HistoricalReader<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}
impl<B: WriteBackend> ActiveWriter<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}
impl<B: ReadBackend> sealed::Role for HistoricalReader<B> {}
impl<B: WriteBackend> sealed::Role for ActiveWriter<B> {}

impl<B: ReadBackend> ReadKeys for HistoricalReader<B> {
    type Error = B::Error;
    fn read(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
        version: u32,
    ) -> Result<&[u8; 32], Self::Error> {
        self.backend.select(owner, workspace, version)
    }
}

impl<B: WriteBackend> WriteKeys for ActiveWriter<B> {
    type Error = B::Error;
    fn for_current_write(
        &self,
        owner: [u8; 16],
        workspace: [u8; 16],
    ) -> Result<(u32, &[u8; 32]), Self::Error> {
        self.backend.current(owner, workspace)
    }
}
